//! Synthetic production egui import → reopen → production export boundary.
use super::*;
const ANLZ_A: &str = "/PIONEER/USBANLZ/P001/0000002A/ANLZ0000.DAT";
const ANLZ_B: &str = "/PIONEER/USBANLZ/P001/00000007/ANLZ0000.DAT";
include!("../../../crates/oxidj-cli/tests/support/library_pdb_fixture.rs");

fn review(shell: &mut Shell, source: &Path, destination: &Path) {
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    shell.click(find(&nodes(&menu), "Import Rekordbox PDB…").2.center());
    shell.app.pdb_source_path = source.display().to_string();
    shell.app.pdb_destination_path = destination.display().to_string();
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Review import…").2.center());
    shell.settle();
    assert!(shell.app.pdb_pending.is_some());
    assert!(!destination.exists() || shell.app.pdb_job.is_none());
}
fn confirm(shell: &mut Shell) {
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Confirm import").2.center());
}
fn finish(shell: &mut Shell) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while shell.app.pdb_job.is_some() {
        assert!(std::time::Instant::now() < deadline, "import worker timed out");
        shell.settle();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
fn source(root: &Path) -> PathBuf {
    let pdb = root.join("device/PIONEER/rekordbox/export.pdb");
    std::fs::create_dir_all(pdb.parent().unwrap()).unwrap();
    std::fs::write(&pdb, synthetic_pdb(None)).unwrap();
    pdb
}

#[test]
fn pdb_import_confirm_cancel_and_background_completion_preserve_active_state() {
    let (root, db) = catalog_root("pdb-import-success");
    let pdb = source(&root);
    let source_before = std::fs::read(&pdb).unwrap();
    let destination = root.join("named-new-catalog.sqlite");
    let mut shell = Shell::new(&db, 1440.0);
    shell.app.search = "keep search".into();
    let active_before = Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap();
    let active_bytes = std::fs::read(&db).unwrap();
    review(&mut shell, &pdb, &destination);
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Cancel").2.center());
    assert!(shell.app.pdb_pending.is_none());
    assert!(shell.app.pdb_job.is_none());
    assert!(!destination.exists());
    review(&mut shell, &pdb, &destination);
    let gate = std::sync::Arc::new(std::sync::Barrier::new(2));
    shell.app.pdb_gate = Some(gate.clone());
    confirm(&mut shell);
    // A deliberately held worker does not block egui frames or ordinary controls.
    assert!(shell.app.pdb_job.is_some());
    assert!(!destination.exists());
    let out = shell.settle();
    shell.click(find(&nodes(&out), "Close").2.center());
    assert!(!shell.app.pdb_import_open);
    let menu = shell.click(shell.app.shell.more.unwrap().center());
    let out = shell.click(find(&nodes(&menu), "Import Rekordbox PDB…").2.center());
    assert!(!nodes(&out).iter().any(|n| n.1.as_deref() == Some("Confirm import")));
    shell.click(find(&nodes(&out), "Close").2.center());
    assert!(shell.app.pdb_job.is_some(), "reopening must retain the original receiver");
    let shown = shell.app.show_nav;
    shell.click(shell.app.shell.toggles.unwrap()[0].center());
    assert_eq!(shell.app.show_nav, !shown);
    assert!(shell.app.pdb_job.is_some());
    gate.wait();
    finish(&mut shell);
    assert!(shell.app.notice.starts_with("Created catalog"), "{}", shell.app.notice);
    assert!(shell.app.notice.contains("3 tracks, 2 folders, 4 playlists, 6 entries"));
    assert!(shell.app.notice.contains(destination.to_str().unwrap()));
    assert!(shell.app.notice.contains("--db"));
    assert_eq!(shell.app.db_path, db);
    assert_eq!(shell.app.search, "keep search");
    assert_eq!(std::fs::read(&db).unwrap(), active_bytes);
    assert_eq!(Catalog::open_read_only(&db).unwrap().library_snapshot().unwrap(), active_before);
    assert_eq!(std::fs::read(&pdb).unwrap(), source_before);
    assert_eq!(std::fs::read_dir(pdb.parent().unwrap()).unwrap().count(), 1);
    assert!(!root.read_dir().unwrap().any(|e| e.unwrap().file_name().to_string_lossy().starts_with(".oxidj-import-")));
    drop(shell);

    // Fresh app opens the exact artifact made by the UI job, without rescanning.
    let snapshot = Catalog::open_read_only(&destination).unwrap().library_snapshot().unwrap();
    assert_eq!(snapshot.tracks.len(), 3);
    assert_eq!(snapshot.tracks[0].title, "Song A");
    assert_eq!(snapshot.tracks[0].comments.as_deref(), Some("Synth <c> & \"d\""));
    assert_eq!(snapshot.tracks[0].key.as_deref(), Some("11B"));
    assert_eq!(snapshot.tracks[0].rating_stars, Some(5));
    use oxidj_library::LibraryNode::{Folder, Playlist};
    assert_eq!(snapshot.roots, vec![
        Playlist { name: "Top".into(), members: vec![1, 0, 0] },
        Folder { name: "Folder".into(), children: vec![
            Playlist { name: "Inner First".into(), members: vec![2] },
            Playlist { name: "Inner Second".into(), members: vec![0, 1] },
            Folder { name: "Empty Folder".into(), children: vec![] },
        ] },
        Playlist { name: "Empty Playlist".into(), members: vec![] },
    ]);
    let mut reopened = Shell::new(&destination, 1440.0);
    assert!(reopened.app.imported_navigation.is_ok());
    let xml = root.join("ui-export.xml");
    let menu = reopened.click(reopened.app.shell.more.unwrap().center());
    let out = reopened.click(find(&nodes(&menu), "Export catalog XML…").2.center());
    reopened.app.catalog_export_path = xml.display().to_string();
    reopened.click(find(&nodes(&out), "Export catalog XML").2.center());
    assert!(reopened.app.notice.starts_with("Catalog exported"), "{}", reopened.app.notice);
    // Independent whole-tree oracle: values fixed by the shared PDB builder,
    // not by production mapping, exporter, or preview. Mutation must be detected.
    let oracle = r#"import sys, xml.etree.ElementTree as E
expected = E.fromstring('''<DJ_PLAYLISTS Version="1.0.0"><PRODUCT Name="OxiDJ" Version="0.1.0" Company="OxiDJ"/><COLLECTION Entries="3"><TRACK TrackID="1" Name="Song A" Artist="Synthetic Artist" TotalTime="180" AverageBpm="128.00" Location="file://localhost/Contents/Song%20A.mp3" Genre="Synth Genre" Label="Synth Label" Tonality="11B" Comments="Synth &lt;c&gt; &amp; &quot;d&quot;" DateAdded="2025-12-31" Year="2019" BitRate="320" SampleRate="48000" Size="7340032" TrackNumber="7" DiscNumber="1" Rating="255"/><TRACK TrackID="2" Name="Song &lt;B&gt; &amp; &quot;Q&quot;" TotalTime="61" AverageBpm="90.50" Location="file://localhost/b.wav" BitRate="128"/><TRACK TrackID="3" Name="Song C" Artist="Synthetic Artist" TotalTime="0" AverageBpm="0.00" Location="file://localhost/Contents/C.flac"/></COLLECTION><PLAYLISTS><NODE Type="0" Name="ROOT" Count="3"><NODE Type="1" Name="Top" KeyType="0" Entries="3"><TRACK Key="2"/><TRACK Key="1"/><TRACK Key="1"/></NODE><NODE Type="0" Name="Folder" Count="3"><NODE Type="1" Name="Inner First" KeyType="0" Entries="1"><TRACK Key="3"/></NODE><NODE Type="1" Name="Inner Second" KeyType="0" Entries="2"><TRACK Key="1"/><TRACK Key="2"/></NODE><NODE Type="0" Name="Empty Folder" Count="0"/></NODE><NODE Type="1" Name="Empty Playlist" KeyType="0" Entries="0"/></NODE></PLAYLISTS></DJ_PLAYLISTS>''')
def tree(e): return e.tag, dict(e.attrib), [tree(c) for c in e]
actual = E.parse(sys.argv[1]).getroot()
assert tree(actual) == tree(expected), (E.tostring(actual), E.tostring(expected))
actual.find('./PLAYLISTS/NODE/NODE').remove(actual.find('./PLAYLISTS/NODE/NODE/TRACK'))
assert tree(actual) != tree(expected)
"#;
    let check = std::process::Command::new("python3").args(["-c", oracle]).arg(&xml).output().unwrap();
    assert!(check.status.success(), "{}", String::from_utf8_lossy(&check.stderr));
    assert_eq!(std::fs::read(&pdb).unwrap(), source_before);
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

// The child receives synthetic util-linux command outputs through its private
// PATH. No global environment mutation and no physical/remounted USB is needed.
#[cfg(target_os = "linux")]
#[test]
fn pdb_import_ui_refuses_synthetic_removable_mount() {
    const CHILD: &str = "OXIDJ_TEST_REMOVABLE_IMPORT_CHILD";
    if std::env::var_os(CHILD).is_none() {
        use std::os::unix::fs::PermissionsExt;
        let (root, _) = catalog_root("pdb-removable-command-fixture");
        let bin = root.join("bin");
        std::fs::create_dir(&bin).unwrap();
        for (name, json) in [
            ("findmnt", r#"{"filesystems":[{"fstype":"exfat","sources":["/dev/synthetic-usb"]}]}"#),
            ("lsblk", r#"{"blockdevices":[{"type":"disk","tran":"usb","rm":false,"hotplug":false}]}"#),
        ] {
            let path = bin.join(name);
            std::fs::write(&path, format!("#!/bin/sh\nprintf '%s\\n' '{json}'\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "shell_tests::pdb_import_tests::pdb_import_ui_refuses_synthetic_removable_mount", "--nocapture"])
            .env(CHILD, "1").env("PATH", &bin).output().unwrap();
        assert!(result.status.success(), "{}\n{}", String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
        assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
        std::fs::remove_dir_all(root).unwrap();
        return;
    }
    let (root, db) = catalog_root("pdb-removable-egui");
    let source_dir = root.join("nonstandard-source-device");
    let other = root.join("other-removable-device");
    std::fs::create_dir(&source_dir).unwrap();
    std::fs::create_dir(&other).unwrap();
    std::os::unix::fs::symlink(&other, root.join("alias")).unwrap();
    let pdb = source_dir.join("backup.pdb");
    let bytes = synthetic_pdb(None);
    std::fs::write(&pdb, &bytes).unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let active = std::fs::read(&db).unwrap();
    for dir in [&source_dir, &other, &root.join("alias")] {
        let before = std::fs::read_dir(dir).unwrap().count();
        let destination = dir.join("no.sqlite");
        review(&mut shell, &pdb, &destination);
        confirm(&mut shell); finish(&mut shell);
        assert_eq!(shell.app.notice, format!("PDB import failed: destination must be on verified local non-removable storage (not USB); storage classification failed or destination is unsupported. Current catalog unchanged."));
        assert_eq!(std::fs::read_dir(dir).unwrap().count(), before, "no stage/catalog/sidecars");
        assert_eq!(std::fs::read(&pdb).unwrap(), bytes);
        assert_eq!(std::fs::read(&db).unwrap(), active);
        assert_eq!(shell.app.db_path, db);
        let out = shell.settle();
        shell.click(find(&nodes(&out), "Cancel").2.center());
    }
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pdb_import_ui_surfaces_invalid_source_existing_destinations_and_device_refusals() {
    let (root, db) = catalog_root("pdb-import-errors");
    let pdb = source(&root);
    let good_bytes = std::fs::read(&pdb).unwrap();
    let mut shell = Shell::new(&db, 1440.0);
    let before = std::fs::read(&db).unwrap();
    for (destination, expected) in [
        (db.clone(), "existing catalog path"),
        (root.join("sidecar.sqlite"), "existing catalog path"),
        (root.join("device/no.sqlite"), "outside the source device"),
    ] {
        let sidecar = root.join("sidecar.sqlite-wal");
        std::fs::write(&sidecar, b"foreign sidecar").unwrap();
        review(&mut shell, &pdb, &destination);
        confirm(&mut shell); finish(&mut shell);
        assert!(shell.app.notice.contains(expected), "{}", shell.app.notice);
        assert_eq!(std::fs::read(&sidecar).unwrap(), b"foreign sidecar");
        assert_eq!(std::fs::read(&db).unwrap(), before);
        assert_eq!(std::fs::read(&pdb).unwrap(), good_bytes);
        let out = shell.settle(); shell.click(find(&nodes(&out), "Cancel").2.center());
    }
    std::fs::write(&pdb, b"not a PDB").unwrap();
    let destination = root.join("invalid.sqlite");
    review(&mut shell, &pdb, &destination);
    confirm(&mut shell); finish(&mut shell);
    assert!(shell.app.notice.contains("PDB source was rejected"), "{}", shell.app.notice);
    assert!(!destination.exists());
    assert_eq!(std::fs::read(&pdb).unwrap(), b"not a PDB");
    assert_eq!(std::fs::read(&db).unwrap(), before);
    assert_eq!(shell.app.db_path, db);
    drop(shell);
    std::fs::remove_dir_all(root).unwrap();
}
