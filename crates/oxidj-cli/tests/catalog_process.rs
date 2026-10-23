use oxidj_core as _;
use oxidj_library::Catalog;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(args)
        .output()
        .expect("failed to run oxidj-cli")
}

fn database_path() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "oxidj-catalog-{}-{nonce}.sqlite",
        std::process::id()
    ))
}

#[test]
fn catalog_record_survives_a_fresh_cli_process() {
    let database = database_path();
    let database_arg = database.to_str().expect("temporary path is not UTF-8");

    let init = run(&["catalog", "init", "--db", database_arg]);
    assert_eq!(
        init.status.code(),
        Some(0),
        "init stderr: {:?}",
        init.stderr
    );
    assert!(database.is_file());

    let list = run(&["catalog", "list", "--db", database_arg]);
    assert_eq!(
        list.status.code(),
        Some(0),
        "list stderr: {:?}",
        list.stderr
    );
    assert_eq!(list.stdout, b"TrackId\tPath\tSHA-256\tBytes\n");
    assert!(list.stderr.is_empty());

    let mut catalog = Catalog::open(&database).expect("open catalog through library API");
    let inserted = catalog
        .insert_track("operation-1", "/scratch/synthetic.wav", [0xab; 32], 7)
        .expect("insert synthetic track");
    assert_eq!(inserted.id.as_u64(), 1);
    assert_eq!(
        catalog.read_track(inserted.id).expect("read track"),
        Some(inserted)
    );
    drop(catalog);

    let reopened = run(&["catalog", "list", "--db", database_arg]);
    assert_eq!(
        reopened.status.code(),
        Some(0),
        "list stderr: {:?}",
        reopened.stderr
    );
    assert_eq!(
        reopened.stdout,
        b"TrackId\tPath\tSHA-256\tBytes\n1\t/scratch/synthetic.wav\tabababababababababababababababababababababababababababababababab\t7\n"
    );
    assert!(reopened.stderr.is_empty());

    std::fs::remove_file(database).expect("remove temporary catalog");
}
