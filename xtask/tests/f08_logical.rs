use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f08-logical")
        .join(name)
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(fixture(name)).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// Independently authored literal oracle: it validates artifacts, not production adapter output.
fn check(
    source_bytes: &[u8],
    archive: &Value,
    visible: &Value,
    plan: &Value,
    report: &Value,
) -> Result<(), &'static str> {
    let source: Value = serde_json::from_slice(source_bytes).map_err(|_| "invalid source")?;
    let expected = json!({"colorTableIndex":3,"cueComment":"custom","inDecodingStartFramePosition":0,"inMpegFrameNumber":750000,"inNumberOfSampleInBlock":480000,"inTimeCode":0,"inUsec":10000000,"in150FramePerSec":1500000,"isActiveLoop":false,"kind":0,"opaque_hex":"deadbeef","outDecodingStartFramePosition":0,"outMpegFrameNumber":1500000,"outNumberOfSampleInBlock":960000,"outTimeCode":0,"outUsec":20000000,"out150FramePerSec":3000000});
    if source != expected {
        return Err("source recipe mismatch");
    }
    if archive["source_bytes_hex"] != hex(source_bytes)
        || archive["source_object"] != expected
        || archive["effective_object"] != expected
        || archive["operations"] != json!([])
        || archive["revision"] != 0
        || archive["track_id"] != "t1"
    {
        return Err("archive mismatch");
    }
    if visible["object"] != expected
        || visible["source_bytes_hex"] != hex(source_bytes)
        || visible["report"] != json!([])
        || report != &json!([])
    {
        return Err("visible/report mismatch");
    }
    if plan
        != &json!({"contract_version":"logical-v1","case_id":"F-08","target_format":"onelibrary-synthetic","export_policy":"safe-v1","decision_phase":"pre-write","outcome":"ALLOW_EXPORT","selected_units":["r1"],"withheld_units":[],"report":[],"target_write":"permitted-but-not-performed-by-fixture-harness","no_write_expected":false})
    {
        return Err("plan mismatch");
    }
    Ok(())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn check_provenance(
    provenance: &Value,
    identity: &Value,
    live_checker: &[u8],
    source: &[u8],
) -> Result<(), &'static str> {
    let expected_identity = json!({
        "path": "xtask/tests/f08_logical.rs",
        "version": "f08-logical-checker-v1",
        "sha256": digest(live_checker)
    });
    if identity != &expected_identity || provenance["checker_identity"] != *identity {
        return Err("checker identity mismatch");
    }
    if provenance["case_id"] != "F-08"
        || provenance["classification"] != "logical-only-not-A3-evidence"
        || provenance["source_citation"] != "docs/adr/005-relationship-fixtures.md §3 F-08"
        || provenance["source_version"]
            != "Synthetic F-05 logical object with F-08 alternate-encoding values; not OneLibrary bytes"
        || provenance["source_length_bytes"] != source.len()
        || provenance["source_sha256"] != digest(source)
    {
        return Err("provenance source claims mismatch");
    }

    let expected_artifacts = [
        "case.json",
        "expected-archive.json",
        "expected-plan.json",
        "expected-report.json",
        "expected-visible.json",
        "operations.json",
        "source.bin",
        "checker-identity.json",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let artifacts = provenance["artifacts"]
        .as_object()
        .ok_or("provenance artifacts missing")?;
    if artifacts
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected_artifacts
    {
        return Err("provenance artifact set mismatch");
    }
    let actual = std::fs::read_dir(fixture("."))
        .map_err(|_| "fixture missing")?
        .map(|entry| entry.map_err(|_| "entry unreadable"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>();
    let expected_files = expected_artifacts
        .iter()
        .map(|name| name.to_string())
        .chain(["provenance.json".to_owned()])
        .collect::<BTreeSet<_>>();
    if actual != expected_files {
        return Err("live fixture artifact set mismatch");
    }
    for (name, info) in artifacts {
        let bytes = std::fs::read(fixture(name)).map_err(|_| "provenance artifact missing")?;
        if info["path"] != name.as_str()
            || info["length_bytes"] != bytes.len()
            || info["sha256"] != digest(&bytes)
        {
            return Err("provenance artifact claim mismatch");
        }
    }
    Ok(())
}

#[test]
fn f08_artifacts_provenance_and_no_edit_echo_are_frozen() {
    let source = std::fs::read(fixture("source.bin")).unwrap();
    let identity = read("checker-identity.json");
    let live_checker =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f08_logical.rs"))
            .unwrap();
    let provenance = read("provenance.json");
    check_provenance(&provenance, &identity, &live_checker, &source).unwrap();
    assert_eq!(read("case.json")["case_id"], "F-08");
    assert_eq!(read("operations.json"), json!([]));
    let archive = read("expected-archive.json");
    let visible = read("expected-visible.json");
    let plan = read("expected-plan.json");
    let report = read("expected-report.json");
    check(&source, &archive, &visible, &plan, &report).unwrap();
    assert_eq!(hex(&source), archive["source_bytes_hex"]);
}

#[test]
fn f08_checker_rejects_opaque_byte_changes_and_invented_conversions() {
    let source = std::fs::read(fixture("source.bin")).unwrap();
    let archive = read("expected-archive.json");
    let visible = read("expected-visible.json");
    let plan = read("expected-plan.json");
    let report = read("expected-report.json");
    let provenance = read("provenance.json");
    let identity = read("checker-identity.json");
    let live_checker =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f08_logical.rs"))
            .unwrap();
    for (key, value) in [
        ("path", json!("xtask/tests/other.rs")),
        ("version", json!("drift")),
        ("sha256", json!("wrong")),
    ] {
        let mut drift = identity.clone();
        drift[key] = value;
        assert!(check_provenance(&provenance, &drift, &live_checker, &source).is_err());
    }
    assert!(check_provenance(&provenance, &identity, b"mutated checker", &source).is_err());
    let mut bad_artifact_set = provenance.clone();
    bad_artifact_set["artifacts"]["unexpected.json"] = json!({});
    assert!(check_provenance(&bad_artifact_set, &identity, &live_checker, &source).is_err());
    let mut invented_classification = provenance.clone();
    invented_classification["classification"] = json!("real-onelibrary-evidence");
    assert!(check_provenance(&invented_classification, &identity, &live_checker, &source).is_err());
    let mut invented_citation = provenance.clone();
    invented_citation["source_citation"] = json!("OneLibrary device export");
    assert!(check_provenance(&invented_citation, &identity, &live_checker, &source).is_err());
    let mut extra_artifact = provenance;
    extra_artifact["artifacts"]["invented.json"] = json!({});
    assert!(check_provenance(&extra_artifact, &identity, &live_checker, &source).is_err());
    let mut changed_source: Value = serde_json::from_slice(&source).unwrap();
    changed_source["opaque_hex"] = json!("00deadbeef");
    assert!(
        check(
            &serde_json::to_vec(&changed_source).unwrap(),
            &archive,
            &visible,
            &plan,
            &report
        )
        .is_err()
    );
    let mut changed_archive = archive.clone();
    changed_archive["source_bytes_hex"] = json!("00deadbeef");
    assert!(check(&source, &changed_archive, &visible, &plan, &report).is_err());
    let mut converted = visible.clone();
    converted["object"]["inMpegFrameNumber"] = json!(750001);
    assert!(check(&source, &archive, &converted, &plan, &report).is_err());
    let mut invented_semantics = visible.clone();
    invented_semantics["object"]["kind"] = json!("Loop");
    assert!(check(&source, &archive, &invented_semantics, &plan, &report).is_err());
}
