use oxidj_application as _;
use oxidj_core as _;
use oxidj_library as _;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(args)
        .output()
        .expect("failed to run oxidj-cli")
}

#[test]
fn track_id_process_contract() {
    let no_command = run(&[]);
    assert_eq!(no_command.status.code(), Some(2));
    assert_eq!(
        no_command.stderr,
        b"usage: oxidj-cli <command>\ncommands:\n  rekordbox preview --xml <path>   Read-only Rekordbox XML preview\n  track-id <n>   Construct and echo a domain TrackId (n >= 1)\n  catalog init --db <path>   Initialize local catalog\n  catalog list --db <path>   List local catalog tracks\n  catalog import --dir <directory> --db <path>   Import WAV/FLAC/MP3 references recursively\n  catalog backup --db <source> --to <new-backup>   Create verified backup\n  catalog restore --from <backup> --db <new-destination>   Restore backup\n"
    );

    let success = run(&["track-id", "1"]);
    assert_eq!(success.status.code(), Some(0));
    assert_eq!(success.stdout, b"1\n");
    assert!(success.stderr.is_empty());

    let zero = run(&["track-id", "0"]);
    assert_eq!(zero.status.code(), Some(1));
    assert_eq!(
        zero.stderr,
        b"error: track id must be >= 1 (0 is reserved)\n"
    );

    for raw in ["", "abc", "-1", "18446744073709551616"] {
        let malformed = run(&["track-id", raw]);
        assert_eq!(malformed.status.code(), Some(2), "input: {raw:?}");
        assert_eq!(
            malformed.stderr,
            b"error: argument must be an unsigned integer\n"
        );
    }

    let missing = run(&["track-id"]);
    assert_eq!(missing.status.code(), Some(2));
    assert_eq!(
        missing.stderr,
        b"error: track-id requires a numeric argument\n"
    );

    let unknown = run(&["bogus"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert_eq!(unknown.stderr, b"error: unknown command: bogus\n");
}
