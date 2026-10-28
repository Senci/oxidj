//! Cross-case F-09 fixture inventory audit. Synthetic logical fixtures only.
use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const CASES: &[&str] = &[
    "f01-relocation-logical",
    "f01-conflict-logical",
    "f01-same-path-logical",
    "f01-unconfirmed-logical",
    "f01-metadata-conflict-logical",
    "rekordbox-a1/F-02",
    "f03-logical",
    "f04-compose-logical",
    "f04-replay-logical",
    "f04-analysis-logical",
    "f04-cue-order-logical",
    "f05-logical",
    "f06a-logical",
    "f06b-logical",
    "rekordbox-a1/F-07",
    "f08-logical",
    "f08b-logical",
    "rekordbox-a1/F-10",
    "f11-logical",
    "f12-logical",
];
const PROVENANCE_SHA256: &[(&str, &str)] = &[
    (
        "f01-relocation-logical",
        "27565935557f590e501e547640240adc47c522db16fb3e78c09e8319c6963d15",
    ),
    (
        "f01-conflict-logical",
        "7b44f3abe73fbfadb58b5e45eef59f85bf6725981a798d2f2bb4f4f8fbe25311",
    ),
    (
        "f01-same-path-logical",
        "2ab5c9f9acced0053ee9010fd4c001c4e89b20030b9f9d9d487b2c20da5962aa",
    ),
    (
        "f01-unconfirmed-logical",
        "1c44cddac8f149d28d455e53db358b317eae8ae6f0d33027d8da07f2a8c206a4",
    ),
    (
        "f01-metadata-conflict-logical",
        "b828d6c1f35ef3180cdd5078f13627beb9b863e8b05eb43e7af8b5e19ac36d6f",
    ),
    (
        "rekordbox-a1/F-02",
        "5ad5b7d4f935d22a8a2122ae748d6fa0a53baaaa79658fd895cf4cf6cbc2e5a4",
    ),
    (
        "f03-logical",
        "2a8f1c63ae711f44560ea0de46a44ce8e3c6e1581755cb8a8ae5184924cca9d6",
    ),
    (
        "f04-compose-logical",
        "1364dc6fb1183844805453bad92a58d120a5c06cc3b6661e326ca292b4924532",
    ),
    (
        "f04-replay-logical",
        "64d399ab932d9070024cc4a64e7fb4c626cffd8652324670b956a80d08c9a3a7",
    ),
    (
        "f04-analysis-logical",
        "3ac4b8bbaea197ecfae60c5fca32cdb115847285757b2c220bddd646b66fd68e",
    ),
    (
        "f04-cue-order-logical",
        "49bf2e42d6d6d46575d78ee64c2623098ea41b841265e82ae55d14276ddfbf8c",
    ),
    (
        "f05-logical",
        "5ef3d7e1c039beaee851179e84774b385dc4edb5ce73119015f3d653e2106af9",
    ),
    (
        "f06a-logical",
        "11fc59a28211b58f0a17d47e9de68a44c4eec5106c6dfe521071300a9baae971",
    ),
    (
        "f06b-logical",
        "daed9526da7e9eb795b488065a8b9bdda6fa044fdb68c51e46cd87800e977de6",
    ),
    (
        "rekordbox-a1/F-07",
        "9468c00417367e42fb3523413f6baef723315d5dfdf92845386ba74537793af8",
    ),
    (
        "f08-logical",
        "9d0e721c988b04a757589a3e5d0faa121c0aca7a42bba2cc9a04bbcec5a2f652",
    ),
    (
        "f08b-logical",
        "34fd6eec16e0e1955181f7580310cdf317eafda2e881beee6bde72297de49e0e",
    ),
    (
        "rekordbox-a1/F-10",
        "97a42baf7b8f32c62071e557498112cba987443f8f957339be598795c1bee6b9",
    ),
    (
        "f11-logical",
        "39c61fa5959fc88452baa026c5c5f69c691cfadefe54e9ca68c840217c5c9f22",
    ),
    (
        "f12-logical",
        "85dd88e1593a1d557370444b2b3c9222f127d28d18104ee5ba0b4709564d77b6",
    ),
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn f01_to_f12_fixture_artifact_and_checker_inventory_is_frozen() {
    let root = root();
    assert_eq!(CASES.len(), 20);
    for case in CASES {
        let dir = root.join(case);
        let provenance_path = dir.join("provenance.json");
        let provenance_bytes = fs::read(&provenance_path).unwrap();
        let expected_provenance = PROVENANCE_SHA256
            .iter()
            .find(|(name, _)| *name == *case)
            .unwrap()
            .1;
        assert_eq!(
            sha(&provenance_bytes),
            expected_provenance,
            "frozen provenance drift in {case}"
        );
        let provenance: Value = serde_json::from_slice(&provenance_bytes).unwrap();
        let listed = provenance["artifacts"]
            .as_object()
            .expect("artifact digest map");
        let mut expected: BTreeSet<String> = listed.keys().cloned().collect();
        let identity_path = if dir.join("checker-identity.json").is_file() {
            dir.join("checker-identity.json")
        } else {
            dir.parent().unwrap().join("checker-identity.json")
        };
        if dir.join("checker-identity.json").is_file() {
            expected.insert("checker-identity.json".into());
        }
        expected.insert("provenance.json".into());
        let actual: BTreeSet<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(actual, expected, "artifact set drift in {case}");
        assert!(
            listed.contains_key("source.bin"),
            "missing source digest in {case}"
        );
        let identity = json(&identity_path);
        let (path_key, version_key, digest_key) = if identity.get("path").is_some() {
            ("path", "version", "sha256")
        } else {
            ("checker_path", "checker_version", "checker_sha256")
        };
        let checker_path = identity[path_key].as_str().expect("checker path");
        let checker = fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join(checker_path),
        )
        .expect("independent checker source");
        assert!(
            identity[version_key]
                .as_str()
                .filter(|s| !s.is_empty())
                .is_some(),
            "checker version in {case}"
        );
        assert_eq!(
            identity[digest_key],
            sha(&checker),
            "checker identity drift in {case}"
        );
        assert_eq!(
            provenance["checker_identity"], identity,
            "provenance identity drift in {case}"
        );
        for (name, record) in listed {
            let bytes =
                fs::read(dir.join(name)).unwrap_or_else(|_| panic!("missing {case}/{name}"));
            assert_eq!(record["path"], name.as_str(), "path drift in {case}/{name}");
            assert_eq!(
                record["length_bytes"].as_u64(),
                Some(bytes.len() as u64),
                "length drift in {case}/{name}"
            );
            assert_eq!(
                record["sha256"],
                sha(&bytes),
                "digest drift in {case}/{name}"
            );
        }
    }
}

#[test]
fn f09_required_negative_controls_have_executable_regressions() {
    // ADR-005 §2 F-09: index zero; units; House order; original Intro; omitted
    // F-10 point; restored withheld c38; opaque byte; missing required report.
    let tests = [
        (
            "index-zero",
            "f03_logical",
            "f03_mutation_controls_reject_index_end_collapse_and_grid",
        ),
        (
            "unit-swap",
            "f03_logical",
            "f03_mutation_controls_reject_index_end_collapse_and_grid",
        ),
        (
            "House-order",
            "f02_sibling_playlist_reorder_oracle",
            "independently_decodes_sibling_reorder_and_rejects_output_mutations",
        ),
        (
            "original-Intro",
            "f04_cue_order_logical",
            "frozen_artifacts_independent_checker_and_mutation_controls",
        ),
        (
            "F-10-omitted-point",
            "rekordbox_xml_a1",
            "f10_checker_rejects_changed_missing_or_reordered_tempo_points",
        ),
        (
            "restored-c38",
            "f12_logical",
            "f12_mutations_reject_archive_renumber_resurrection_and_bad_loss_entries",
        ),
        (
            "opaque-byte",
            "f05_logical",
            "f05_checker_rejects_loop_inference_raw_mutation_and_missing_block_entry",
        ),
        (
            "missing-report-entry",
            "f06b_logical",
            "f06b_mutations_reject_loss_report_changes_and_resurrection",
        ),
    ];
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    for (control, test, identifier) in tests {
        let source = fs::read_to_string(source_root.join(format!("{test}.rs")))
            .unwrap_or_else(|_| panic!("missing executable {control} control source: {test}"));
        let test_declaration = format!("#[test]\nfn {identifier}()");
        assert!(
            source.contains(&test_declaration),
            "missing executable {control} control test {test}::{identifier}"
        );
    }
}
