use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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

#[test]
fn f08_artifacts_provenance_and_no_edit_echo_are_frozen() {
    let source = std::fs::read(fixture("source.bin")).unwrap();
    let provenance = read("provenance.json");
    assert_eq!(provenance["source_sha256"], digest(&source));
    assert_eq!(provenance["source_length_bytes"], source.len());
    for (name, info) in provenance["artifacts"].as_object().unwrap() {
        let bytes = std::fs::read(fixture(name)).unwrap();
        assert_eq!(info["path"], name.as_str());
        assert_eq!(info["length_bytes"], bytes.len(), "{name}");
        assert_eq!(info["sha256"], digest(&bytes), "{name}");
    }
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
