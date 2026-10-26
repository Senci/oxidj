use oxidj_application as _;
use oxidj_core as _;
use oxidj_library as _;
use std::process::Command;

#[test]
fn preview_prints_tracks_ordered_playlists_tempos_and_marks() {
    let fixture = format!(
        "{}/../../xtask/tests/fixtures/rekordbox-a1/F-07/source.bin",
        env!("CARGO_MANIFEST_DIR")
    );
    let out = Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(["rekordbox", "preview", "--xml", &fixture])
        .output()
        .expect("run CLI");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("700\tTempo cue independence\tfile://localhost/music/tempo.flac"));
    assert!(stdout.contains("TEMPO\t10\t128\t4/4\t1"));
    assert!(stdout.contains("POSITION_MARK\tIntro\t0\t5\t0\tNone"));
    assert!(stdout.contains("Playlists:"));
}
