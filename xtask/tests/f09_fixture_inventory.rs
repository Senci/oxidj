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

// Each case pins its checker source and baseline test, including the shared A1 test.
const CASES: &[(&str, &str, &str)] = &[
    (
        "f01-relocation-logical",
        "xtask/tests/f01_relocation_logical.rs",
        "frozen_artifacts_provenance_and_independent_checker",
    ),
    (
        "f01-conflict-logical",
        "xtask/tests/f01_conflict_logical.rs",
        "frozen_artifact_set_provenance_and_contract",
    ),
    (
        "f01-same-path-logical",
        "xtask/tests/f01_same_path_logical.rs",
        "frozen_artifact_set_provenance_and_contract",
    ),
    (
        "f01-unconfirmed-logical",
        "xtask/tests/f01_unconfirmed_logical.rs",
        "frozen_artifact_set_provenance_and_contract",
    ),
    (
        "f01-metadata-conflict-logical",
        "xtask/tests/f01_metadata_conflict_logical.rs",
        "frozen_artifacts_and_literal_checker",
    ),
    (
        "rekordbox-a1/F-02",
        "xtask/tests/rekordbox_xml_a1.rs",
        "a1_fixture_bytes_match_frozen_provenance_and_goldens",
    ),
    (
        "f03-logical",
        "xtask/tests/f03_logical.rs",
        "f03_artifacts_frozen_and_independently_equal",
    ),
    (
        "f04-compose-logical",
        "xtask/tests/f04_compose_logical.rs",
        "frozen_artifacts_and_literal_contract",
    ),
    (
        "f04-replay-logical",
        "xtask/tests/f04_replay_logical.rs",
        "frozen_contract_and_artifacts",
    ),
    (
        "f04-analysis-logical",
        "xtask/tests/f04_analysis_logical.rs",
        "frozen_artifacts_and_literal_state",
    ),
    (
        "f04-cue-order-logical",
        "xtask/tests/f04_cue_order_logical.rs",
        "frozen_artifacts_independent_checker_and_mutation_controls",
    ),
    (
        "f05-logical",
        "xtask/tests/f05_logical.rs",
        "f05_frozen_artifacts_and_separate_target_decisions_match",
    ),
    (
        "f06a-logical",
        "xtask/tests/f06a_logical.rs",
        "f06a_frozen_logical_fixture_matches_independent_contract",
    ),
    (
        "f06b-logical",
        "xtask/tests/f06b_logical.rs",
        "f06b_frozen_logical_fixture_matches_independent_contract",
    ),
    (
        "rekordbox-a1/F-07",
        "xtask/tests/rekordbox_xml_a1.rs",
        "a1_fixture_bytes_match_frozen_provenance_and_goldens",
    ),
    (
        "f08-logical",
        "xtask/tests/f08_logical.rs",
        "f08_artifacts_provenance_and_no_edit_echo_are_frozen",
    ),
    (
        "f08b-logical",
        "xtask/tests/f08b_logical.rs",
        "f08b_artifacts_and_provenance_are_frozen",
    ),
    (
        "rekordbox-a1/F-10",
        "xtask/tests/rekordbox_xml_a1.rs",
        "a1_fixture_bytes_match_frozen_provenance_and_goldens",
    ),
    (
        "f11-logical",
        "xtask/tests/f11_logical.rs",
        "f11_logical_artifacts_and_provenance_are_frozen",
    ),
    (
        "f12-logical",
        "xtask/tests/f12_logical.rs",
        "f12_artifacts_provenance_and_independent_contract_are_frozen",
    ),
];
const PROVENANCE_SHA256: &[(&str, &str)] = &[
    (
        "f01-relocation-logical",
        "ac4efec70b21bbc0951041e2cb8d4cd4d271b118fa82f6fb8c4dbc221c732edb",
    ),
    (
        "f01-conflict-logical",
        "6974fa52e8d6994d0fee450b20598734581c0746af9c243527074ecd4a69e1b3",
    ),
    (
        "f01-same-path-logical",
        "c9e36ffff34e994a151b7102d4310c376455c10f3adcb1b5ec4a77887a818dce",
    ),
    (
        "f01-unconfirmed-logical",
        "5b991b0fbe0dd12935fe143d471b23742f0f3c9034f9e4ca9962e25a1e987d21",
    ),
    (
        "f01-metadata-conflict-logical",
        "c485bc27633d5da9b518cf43b8e0120812c3e4d6d7f73372feba864e584995ac",
    ),
    (
        "rekordbox-a1/F-02",
        "510ed3b41b209a304788423eb3fa5dbab46e450e341ecd306654fb9327705d09",
    ),
    (
        "f03-logical",
        "adeb2f725670bf974985c9be9f2ef97f02baf4527080621c77d20cecaa0cec42",
    ),
    (
        "f04-compose-logical",
        "18cf7585e679798a647b0d52ff3ec7e234bdf64cb9c385aa11d7ebe6549393cb",
    ),
    (
        "f04-replay-logical",
        "e81f132aac738e65dd7db740dd9b68990299ea4e86bbb9de309f8b6015fe69a1",
    ),
    (
        "f04-analysis-logical",
        "a232d00b739655c647010cbe9204cd09ca9cc67f6d9316031488b1c65074894e",
    ),
    (
        "f04-cue-order-logical",
        "bc55431a4c177628036dbd7d354428004a90ac626a83f900ce30900804b6fb44",
    ),
    (
        "f05-logical",
        "4c20b9b4ab2174a5a02de9650a4a5cb28c5d4a186aca3c955dc2b884a8b64df6",
    ),
    (
        "f06a-logical",
        "f394c99eca32df05b8816cdd144fab443f713ee266fc33f62637025e710b0785",
    ),
    (
        "f06b-logical",
        "536e25d7bb67942ee51ca4533a1eba83af2074d112723cb8cb9a1ee741e6f046",
    ),
    (
        "rekordbox-a1/F-07",
        "860f0f007ea620936bc9ef185340af5967b0a1082c84f5ee016eda7642f2ba73",
    ),
    (
        "f08-logical",
        "862ebf323a258514805a4e0f7c101374d732a21bd5f5dca96f38c39018113fab",
    ),
    (
        "f08b-logical",
        "d49e374576a0078b07979a59c947ac4bcf2e523a18170c98f3574f0ba129d5a8",
    ),
    (
        "rekordbox-a1/F-10",
        "a06bcb0bfd34e8b3974ef87afe584d48be66bf7f372c3fa25145d0b0398e916b",
    ),
    (
        "f11-logical",
        "2cd7ebab7e9d7404a81a3da75c117d1b77a0a6c206c2278a47aabf125d5a07b3",
    ),
    (
        "f12-logical",
        "d376996cea50d4ef88bed4d6faee6d109f7ffd7e82379dac8adbb0587d66bff6",
    ),
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}
fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}
fn json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn f01_to_f12_fixture_artifact_and_checker_inventory_is_frozen() {
    let root = root();
    assert_eq!(CASES.len(), 20);
    for &(case, expected_checker_path, identifier) in CASES {
        let dir = root.join(case);
        let provenance_path = dir.join("provenance.json");
        let provenance_bytes = fs::read(&provenance_path).unwrap();
        let expected_provenance = PROVENANCE_SHA256
            .iter()
            .find(|(name, _)| *name == case)
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
        assert_eq!(
            checker_path, expected_checker_path,
            "checker source drift in {case} for test {identifier}"
        );
        let checker = fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join(checker_path),
        )
        .unwrap_or_else(|_| {
            panic!("missing checker source {checker_path} for {case}::{identifier}")
        });
        let test_declaration = format!("#[test]\nfn {identifier}()");
        assert!(
            std::str::from_utf8(&checker)
                .unwrap()
                .contains(&test_declaration),
            "missing executable checker test for {case}: {checker_path}::{identifier}"
        );
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
            "f12_logical",
            "f12_checker_rejects_source_missing_index_zero",
        ),
        (
            "unit-swap",
            "f03_logical",
            "f03_mutation_controls_reject_index_end_collapse_and_grid",
        ),
        (
            "House-order",
            "f02_playlist_member_reorder_oracle",
            "independently_decodes_member_reorder_and_rejects_semantic_mutations",
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
            "f08_logical",
            "f08_checker_rejects_opaque_byte_changes_and_invented_conversions",
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
