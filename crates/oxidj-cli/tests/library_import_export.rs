//! M7 slice, synthetic-only: PDB → `catalog import-library` → new catalog →
//! `catalog export-library-xml` → new XML, checked by an independent quick-xml decoder
//! against values hard-coded from the fixture definition below.
use oxidj_application as _;
use oxidj_core as _;
use oxidj_library as _;
use quick_xml::{Reader, events::Event};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn scratch() -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "oxidj-library-import-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).unwrap();
    dir
}

fn cli(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("run CLI")
}

fn s(value: &str) -> &std::ffi::OsStr {
    std::ffi::OsStr::new(value)
}

// --- synthetic DeviceSQL builder (one page per table, up to 16 rows per page) ---

fn encoded_string(value: &str) -> Vec<u8> {
    assert!(value.is_ascii() && value.len() < 127);
    let mut encoded = vec![((value.len() + 1) * 2 + 1) as u8];
    encoded.extend_from_slice(value.as_bytes());
    encoded
}

fn add_table(bytes: &mut [u8], table_index: usize, table_type: u32, rows: &[Vec<u8>]) {
    assert!(rows.len() <= 16);
    let page_index = table_index + 1;
    let dir = 0x1c + table_index * 16;
    bytes[dir..dir + 4].copy_from_slice(&table_type.to_le_bytes());
    bytes[dir + 8..dir + 12].copy_from_slice(&(page_index as u32).to_le_bytes());
    bytes[dir + 12..dir + 16].copy_from_slice(&(page_index as u32).to_le_bytes());
    let page = page_index * 4096;
    bytes[page + 0x18] = rows.len() as u8;
    let mut heap = 0usize;
    for (i, row) in rows.iter().enumerate() {
        bytes[page + 0x28 + heap..page + 0x28 + heap + row.len()].copy_from_slice(row);
        let slot = page + 4096 - 6 - 2 * i;
        bytes[slot..slot + 2].copy_from_slice(&(heap as u16).to_le_bytes());
        heap += (row.len() + 3) & !3;
    }
    let present = ((1u32 << rows.len()) - 1) as u16;
    bytes[page + 4092..page + 4094].copy_from_slice(&present.to_le_bytes());
}

fn track_row(
    id: u32,
    artist_id: u32,
    tempo: u32,
    duration: u16,
    title: &str,
    path: &str,
) -> Vec<u8> {
    let mut row = vec![0u8; 0x88];
    row[0x48..0x4c].copy_from_slice(&id.to_le_bytes());
    row[0x44..0x48].copy_from_slice(&artist_id.to_le_bytes());
    row[0x38..0x3c].copy_from_slice(&tempo.to_le_bytes());
    row[0x54..0x56].copy_from_slice(&duration.to_le_bytes());
    let mut strings = vec![String::new(); 21];
    strings[17] = title.into();
    strings[20] = path.into();
    let mut offset = 0x88u16;
    let mut data = Vec::new();
    for (index, value) in strings.iter().enumerate() {
        row[0x5e + index * 2..0x60 + index * 2].copy_from_slice(&offset.to_le_bytes());
        let encoded = encoded_string(value);
        offset += encoded.len() as u16;
        data.extend_from_slice(&encoded);
    }
    row.extend(data);
    row
}

fn artist_row(id: u32, name: &str) -> Vec<u8> {
    let mut row = vec![0u8; 10];
    row[0..2].copy_from_slice(&0x60u16.to_le_bytes());
    row[4..8].copy_from_slice(&id.to_le_bytes());
    row[9] = 10;
    row.extend(encoded_string(name));
    row
}

fn node_row(parent: u32, id: u32, sort: u32, is_folder: bool, name: &str) -> Vec<u8> {
    let mut row = vec![0u8; 20];
    row[0..4].copy_from_slice(&parent.to_le_bytes());
    row[8..12].copy_from_slice(&sort.to_le_bytes());
    row[12..16].copy_from_slice(&id.to_le_bytes());
    row[16..20].copy_from_slice(&u32::from(is_folder).to_le_bytes());
    row.extend(encoded_string(name));
    row
}

fn entry_row(index: u32, track_id: u32, playlist_id: u32) -> Vec<u8> {
    let mut row = vec![0u8; 12];
    row[0..4].copy_from_slice(&index.to_le_bytes());
    row[4..8].copy_from_slice(&track_id.to_le_bytes());
    row[8..12].copy_from_slice(&playlist_id.to_le_bytes());
    row
}

/// 3 tracks / 2 folders / 4 playlists / 6 memberships. Sibling order comes from the
/// sort keys (not ids or row order); member order from entry_index (rows shuffled).
fn synthetic_pdb(extra_entry: Option<Vec<u8>>) -> Vec<u8> {
    let mut bytes = vec![0u8; 5 * 4096];
    bytes[4..8].copy_from_slice(&4096u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&4u32.to_le_bytes());
    add_table(
        &mut bytes,
        0,
        0,
        &[
            track_row(42, 1, 12_800, 180, "Song A", "/Contents/Song A.mp3"),
            track_row(
                7,
                0,
                9_050,
                61,
                "Song <B> & \"Q\"",
                "file://localhost/b.wav",
            ),
            track_row(9, 1, 0, 0, "Song C", "/Contents/C.flac"),
        ],
    );
    add_table(&mut bytes, 1, 2, &[artist_row(1, "Synthetic Artist")]);
    add_table(
        &mut bytes,
        2,
        7,
        &[
            node_row(0, 10, 2, true, "Folder"),
            node_row(10, 11, 1, false, "Inner Second"),
            node_row(10, 12, 0, false, "Inner First"),
            node_row(0, 13, 1, false, "Top"),
            node_row(10, 14, 2, true, "Empty Folder"),
            node_row(0, 15, 3, false, "Empty Playlist"),
        ],
    );
    let mut entries = vec![
        entry_row(2, 42, 13),
        entry_row(0, 7, 13),
        entry_row(1, 42, 13),
        entry_row(0, 9, 12),
        entry_row(1, 7, 11),
        entry_row(0, 42, 11),
    ];
    entries.extend(extra_entry);
    add_table(&mut bytes, 3, 8, &entries);
    bytes
}

// --- independent decoder of the exported XML ---

#[derive(Debug, PartialEq)]
enum Node {
    Folder(String, Vec<Node>),
    Playlist(String, Vec<String>),
}

#[derive(Debug, PartialEq)]
struct Decoded {
    collection_entries: String,
    tracks: Vec<Vec<(String, String)>>,
    root: Node,
}

fn attrs(e: &quick_xml::events::BytesStart<'_>) -> Vec<(String, String)> {
    e.attributes()
        .map(|a| {
            let a = a.unwrap();
            (
                String::from_utf8(a.key.as_ref().to_vec()).unwrap(),
                a.decoded_and_normalized_value(quick_xml::XmlVersion::Explicit1_0, e.decoder())
                    .unwrap()
                    .into_owned(),
            )
        })
        .collect()
}

fn get(attrs: &[(String, String)], key: &str) -> String {
    attrs.iter().find(|(k, _)| k == key).unwrap().1.clone()
}

fn decode(bytes: &[u8]) -> Decoded {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut tracks = Vec::new();
    let mut entries = String::new();
    let mut stack: Vec<Node> = Vec::new();
    let mut root = None;
    let mut in_collection = false;
    loop {
        let event = reader.read_event_into(&mut buf).unwrap();
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let a = attrs(e);
                match e.name().as_ref() {
                    b"COLLECTION" => {
                        entries = get(&a, "Entries");
                        in_collection = !empty;
                    }
                    b"TRACK" if in_collection => tracks.push(a),
                    b"TRACK" => match stack.last_mut() {
                        Some(Node::Playlist(_, keys)) => keys.push(get(&a, "Key")),
                        _ => panic!("member outside playlist"),
                    },
                    b"NODE" => {
                        let node = if get(&a, "Type") == "0" {
                            Node::Folder(get(&a, "Name"), vec![])
                        } else {
                            Node::Playlist(get(&a, "Name"), vec![])
                        };
                        if empty {
                            match stack.last_mut() {
                                Some(Node::Folder(_, children)) => children.push(node),
                                _ => panic!("orphan node"),
                            }
                        } else {
                            stack.push(node);
                        }
                    }
                    _ => {}
                }
            }
            Event::End(ref e) => match e.name().as_ref() {
                b"COLLECTION" => in_collection = false,
                b"NODE" => {
                    let node = stack.pop().unwrap();
                    match stack.last_mut() {
                        Some(Node::Folder(_, children)) => children.push(node),
                        None => root = Some(node),
                        _ => panic!("node inside playlist"),
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Decoded {
        collection_entries: entries,
        tracks,
        root: root.unwrap(),
    }
}

fn kv(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn expected() -> Decoded {
    let p = |name: &str, keys: &[&str]| {
        Node::Playlist(name.into(), keys.iter().map(|k| k.to_string()).collect())
    };
    Decoded {
        collection_entries: "3".into(),
        tracks: vec![
            kv(&[
                ("TrackID", "1"),
                ("Name", "Song A"),
                ("Artist", "Synthetic Artist"),
                ("TotalTime", "180"),
                ("AverageBpm", "128.00"),
                ("Location", "file://localhost/Contents/Song%20A.mp3"),
            ]),
            kv(&[
                ("TrackID", "2"),
                ("Name", "Song <B> & \"Q\""),
                ("TotalTime", "61"),
                ("AverageBpm", "90.50"),
                ("Location", "file://localhost/b.wav"),
            ]),
            kv(&[
                ("TrackID", "3"),
                ("Name", "Song C"),
                ("Artist", "Synthetic Artist"),
                ("TotalTime", "0"),
                ("AverageBpm", "0.00"),
                ("Location", "file://localhost/Contents/C.flac"),
            ]),
        ],
        root: Node::Folder(
            "ROOT".into(),
            vec![
                p("Top", &["2", "1", "1"]),
                Node::Folder(
                    "Folder".into(),
                    vec![
                        p("Inner First", &["3"]),
                        p("Inner Second", &["1", "2"]),
                        Node::Folder("Empty Folder".into(), vec![]),
                    ],
                ),
                p("Empty Playlist", &[]),
            ],
        ),
    }
}

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn assert_aggregate_only(text: &str) {
    for private in [
        "Song",
        "Synthetic Artist",
        "Empty Folder",
        "Empty Playlist",
        "Contents",
        "Inner",
        ".mp3",
        "oxidj-library-import",
    ] {
        assert!(
            !text.contains(private),
            "report leaked a source value: {text}"
        );
    }
}

#[test]
fn pdb_library_imports_into_new_catalog_and_exports_verified_xml_without_touching_source() {
    let dir = scratch();
    let pdb = dir.join("export.pdb");
    let source = synthetic_pdb(None);
    std::fs::write(&pdb, &source).unwrap();
    let catalog = dir.join("catalog.sqlite");
    let xml = dir.join("out.xml");

    let import = cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        catalog.as_os_str(),
    ]);
    let report = stdout(&import);
    assert_aggregate_only(&report);
    for line in [
        "Tracks: 3",
        "Folders: 2",
        "Playlists: 4",
        "Memberships: 6",
        "LocationsUriEncoded: 2",
        "SourceUnchanged: true",
        "CatalogMatchesSource: true",
        "NotCarried\tTRACK@Genre\t3",
        "NotCarried\tTRACK@DateAdded\t3",
        "NotCarried\tTRACK/POSITION_MARK\t3",
        "NotCarried\tTRACK/TEMPO\t3",
        "AbsentInSource\tArtist\t1",
        "AbsentInSource\tAlbum\t3",
    ] {
        assert!(
            report.lines().any(|l| l == line),
            "missing {line:?} in {report}"
        );
    }

    let export = cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        xml.as_os_str(),
    ]);
    let report = stdout(&export);
    assert_aggregate_only(&report);
    for line in [
        "Tracks: 3",
        "Folders: 2",
        "Playlists: 4",
        "Memberships: 6",
        "PreviewVerified: true",
    ] {
        assert!(
            report.lines().any(|l| l == line),
            "missing {line:?} in {report}"
        );
    }
    let bytes = std::fs::read(&xml).unwrap();
    assert_eq!(decode(&bytes), expected());

    // The existing read-only XML preview agrees on aggregates.
    let preview = oxidj_application::preview_rekordbox_xml(&xml).unwrap();
    assert_eq!(preview.tracks.len(), 3);
    assert_eq!(preview.playlists.len(), 4);
    assert_eq!(preview.folders.len(), 3); // ROOT + 2
    assert_eq!(
        preview
            .playlists
            .iter()
            .map(|p| p.tracks.len())
            .sum::<usize>(),
        6
    );

    assert_eq!(
        std::fs::read(&pdb).unwrap(),
        source,
        "source must be byte-identical"
    );

    // New-file-only: a second export to the same path is refused and leaves it intact.
    let again = cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        xml.as_os_str(),
    ]);
    assert!(!again.status.success());
    assert!(
        String::from_utf8_lossy(&again.stderr).contains("refusing to overwrite existing output")
    );
    assert_eq!(std::fs::read(&xml).unwrap(), bytes);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn import_and_export_fail_closed_without_creating_or_replacing_files() {
    let dir = scratch();
    let pdb = dir.join("export.pdb");
    let source = synthetic_pdb(None);
    std::fs::write(&pdb, &source).unwrap();

    // Existing catalog path: refused, untouched.
    let existing = dir.join("existing.sqlite");
    std::fs::write(&existing, b"sentinel").unwrap();
    let out = cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        existing.as_os_str(),
    ]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("refusing to reuse an existing catalog path")
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"sentinel");

    // Malformed source and dangling membership: refused, no catalog created.
    let malformed = dir.join("malformed.pdb");
    std::fs::write(&malformed, b"not a pdb").unwrap();
    let dangling = dir.join("dangling.pdb");
    std::fs::write(&dangling, synthetic_pdb(Some(entry_row(3, 99, 13)))).unwrap();
    for (src, reason) in [
        (
            &malformed,
            "PDB source was rejected by the read-only preview",
        ),
        (&dangling, "playlist member references a missing track"),
    ] {
        let catalog = dir.join("never.sqlite");
        let out = cli(&[
            s("catalog"),
            s("import-library"),
            s("--pdb"),
            src.as_os_str(),
            s("--catalog"),
            catalog.as_os_str(),
        ]);
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains(reason), "{stderr}");
        assert_aggregate_only(&stderr);
        assert!(!catalog.exists());
        assert!(!Path::new(&format!("{}-wal", catalog.display())).exists());
    }

    // Requested OneLibrary cross-check that cannot be read: refused before any catalog.
    let catalog = dir.join("never.sqlite");
    let missing = dir.join("exportLibrary.db");
    let out = cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        catalog.as_os_str(),
        s("--onelibrary-db"),
        missing.as_os_str(),
        s("--key-stdin"),
    ]);
    assert!(!out.status.success());
    assert!(!catalog.exists());

    // Export: missing catalog is refused and not created; existing output untouched.
    let xml = dir.join("out.xml");
    let out = cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        catalog.as_os_str(),
        s("--output"),
        xml.as_os_str(),
    ]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("catalog does not exist"));
    assert!(!catalog.exists() && !xml.exists());

    let good = dir.join("good.sqlite");
    stdout(&cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        good.as_os_str(),
    ]));
    std::fs::write(&xml, b"keep").unwrap();
    let out = cli(&[
        s("catalog"),
        s("export-library-xml"),
        s("--catalog"),
        good.as_os_str(),
        s("--output"),
        xml.as_os_str(),
    ]);
    assert!(!out.status.success());
    assert_eq!(std::fs::read(&xml).unwrap(), b"keep");

    // A second import into the populated catalog is refused (new catalog only).
    let out = cli(&[
        s("catalog"),
        s("import-library"),
        s("--pdb"),
        pdb.as_os_str(),
        s("--catalog"),
        good.as_os_str(),
    ]);
    assert!(!out.status.success());

    assert_eq!(std::fs::read(&pdb).unwrap(), source);
    std::fs::remove_dir_all(dir).unwrap();
}
