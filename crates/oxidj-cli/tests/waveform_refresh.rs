use oxidj_application as _;
use oxidj_core::waveform::{Peak, WaveformPeaks};
use oxidj_library::{Catalog, LibrarySnapshot, LibraryTrack, VendorWaveform};
use quick_xml as _;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn refresh_cli_upgrades_legacy_peaks_and_preserves_vendor_data() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("oxidj-refresh-{}-{nonce}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let db = root.join("catalog.sqlite");
    let mut catalog = Catalog::open(&db).unwrap();
    let mut tracks = Vec::new();
    for n in 0..2 {
        let path = root.join(format!("{n}.wav"));
        let data: Vec<u8> = (0..44_100_i32)
            .flat_map(|i| (((i * (n + 1)) % 16000) as i16).to_le_bytes())
            .collect();
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36_u32 + data.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&44_100_u32.to_le_bytes());
        wav.extend_from_slice(&88_200_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&data);
        std::fs::write(&path, wav).unwrap();
        let (sha, len) = oxidj_library::file_sha256(&path).unwrap();
        let track = catalog
            .insert_track(&format!("track-{n}"), path.to_str().unwrap(), sha, len)
            .unwrap();
        let band = vec![Peak::new(100, 150).unwrap(); 150];
        let legacy =
            WaveformPeaks::from_base(44_100, 294, 44_100, [band.clone(), band.clone(), band])
                .unwrap();
        catalog.store_waveform(track.id, sha, &legacy).unwrap();
        tracks.push((track, path));
    }
    catalog
        .import_library(&LibrarySnapshot {
            tracks: tracks
                .iter()
                .map(|(_, path)| LibraryTrack {
                    title: "synthetic vendor title".into(),
                    location: format!("file://localhost{}", path.display()),
                    tempo_hundredths_bpm: 12300,
                    vendor_waveforms: vec![VendorWaveform::RekordboxDetail(vec![[3, 1]])],
                    ..LibraryTrack::default()
                })
                .collect(),
            roots: vec![],
        })
        .unwrap();
    let vendor_before = catalog.library_snapshot().unwrap();
    assert_eq!(vendor_before.tracks.len(), 2);
    assert_eq!(vendor_before.tracks[1].title, "synthetic vendor title");
    assert_eq!(vendor_before.tracks[1].tempo_hundredths_bpm, 12300);
    assert_eq!(
        vendor_before.tracks[1].vendor_waveforms,
        vec![VendorWaveform::RekordboxDetail(vec![[3, 1]])]
    );
    let old: Vec<_> = tracks
        .iter()
        .map(|(t, _)| catalog.read_waveform(t.id).unwrap().unwrap())
        .collect();
    drop(catalog);
    let invoke = |extra: &[&str], expected: &str, status: i32| {
        let out = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
            .args(["catalog", "analyze-waveform", "--db", db.to_str().unwrap()])
            .args(extra)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(status), "{:?}", out.stderr);
        assert!(String::from_utf8(out.stdout).unwrap().ends_with(expected));
    };
    let snapshot = || {
        let c = Catalog::open(&db).unwrap();
        assert_eq!(c.library_snapshot().unwrap(), vendor_before);
        tracks
            .iter()
            .map(|(t, _)| c.read_waveform(t.id).unwrap().unwrap())
            .collect::<Vec<_>>()
    };
    invoke(&[], "Analyzed: 0\nUnchanged: 2\nFailed: 0\n", 0);
    assert_eq!(snapshot(), old);
    invoke(
        &["--track-id", "1", "--refresh"],
        "Analyzed: 1\nUnchanged: 0\nFailed: 0\n",
        0,
    );
    let targeted = snapshot();
    assert_eq!(targeted[0].peaks.samples_per_peak(), 37);
    assert_eq!(targeted[1], old[1]);
    invoke(&["--refresh"], "Analyzed: 2\nUnchanged: 0\nFailed: 0\n", 0);
    let bulk = snapshot();
    assert_eq!(bulk[0].peaks.samples_per_peak(), 37);
    assert_eq!(bulk[1].peaks.samples_per_peak(), 37);
    Catalog::open(&db)
        .unwrap()
        .store_waveform(tracks[1].0.id, old[1].source_sha256, &old[1].peaks)
        .unwrap();
    std::fs::remove_file(&tracks[1].1).unwrap();
    invoke(
        &["--track-id", "2", "--refresh"],
        "Analyzed: 0\nUnchanged: 0\nFailed: 1\n",
        1,
    );
    let failed = snapshot();
    assert_eq!(failed[0], bulk[0]);
    assert_eq!(failed[1], old[1]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn help_explains_refresh_scope_and_cost() {
    let out = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8(out.stdout).unwrap();
    assert!(help.contains("all catalog tracks"));
    assert!(help.contains("rereads audio"));
    assert!(help.contains("increases derived storage"));
}
