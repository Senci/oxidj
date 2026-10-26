use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f11-logical")
        .join(name)
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(fixture(name)).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// Independent logical oracle: assertions below are literal contract values and do not
// invoke a production importer, projection, exporter, or ID reconciliation helper.
fn independently_check(
    source: &[u8],
    archived: &Value,
    visible: &Value,
    plan: &Value,
    report: &Value,
) -> Result<(), &'static str> {
    let parsed: Value = serde_json::from_slice(source).map_err(|_| "invalid logical source")?;
    if parsed["cues"] != archived["source_cues"] {
        return Err("archive differs from source");
    }
    let cues = parsed["cues"].as_array().ok_or("cues missing")?;
    if cues.len() != 3 {
        return Err("expected c0/c1/c2");
    }
    let c0 = &cues[0];
    let c1 = &cues[1];
    let c2 = &cues[2];
    if c0["id"] != "c0"
        || c0["kind"] != "HotCue"
        || c0["index"] != 0
        || c0["start_usec"] != 10_000_000
        || c0.as_object().unwrap().contains_key("end_usec")
    {
        return Err("c0 point optionality mismatch");
    }
    if c1["id"] != "c1"
        || c1["kind"] != "Loop"
        || c1["start_usec"] != 15_000_000
        || c1["end_usec"] != 25_000_000
    {
        return Err("c1 range mismatch");
    }
    if c2["id"] != "c2"
        || c2["kind"] != "HotCue"
        || c2["start_usec"] != 0
        || c2.as_object().unwrap().contains_key("end_usec")
    {
        return Err("c2 zero point mismatch");
    }
    let record = archived["records"]
        .as_array()
        .and_then(|records| records.first())
        .cloned()
        .unwrap_or(Value::Null);
    if archived["track_id"] != "t1"
        || archived["revision"] != 0
        || archived["operations"] != json!([])
        || archived["source_cues"] != json!(cues)
        || archived["records"].as_array().map(Vec::len) != Some(1)
        || record["record_id"] != "r-cues"
        || record["track_id"] != "t1"
        || record["source_revision"] != "s1"
        || record["source_namespace"] != "F-11"
        || record["origin"] != "SYNTHETIC"
        || record["source_provenance"]
            != json!({"case_id":"F-11","source_kind":"mixxx-cueinfo-logical","source_version":"logical-v1"})
        || record["effective_revision"] != 0
        || record["overlays"] != json!([])
        || record["analysis"] != json!([])
        || record["cues"] != json!([c0, c1, c2])
    {
        return Err("archive mismatch");
    }
    if visible["cues"] != json!([c0, c1, c2]) || visible["report"] != json!([]) {
        return Err("visible mismatch");
    }
    if plan
        != &json!({"contract_version":"logical-v1","case_id":"F-11","target_format":"mixxx-cueinfo-logical","export_policy":"safe-v1","decision_phase":"pre-write","outcome":"ALLOW_EXPORT","selected_units":["c0","c1","c2"],"withheld_units":[],"report":[],"target_write":"permitted-but-not-performed-by-fixture-harness","no_write_expected":false})
    {
        return Err("plan mismatch");
    }
    if report != &json!([]) {
        return Err("report mismatch");
    }
    Ok(())
}

#[test]
fn f11_logical_artifacts_and_provenance_are_frozen() {
    let source = std::fs::read(fixture("source.bin")).unwrap();
    let provenance = read("provenance.json");
    assert_eq!(provenance["source_sha256"], digest(&source));
    assert_eq!(provenance["source_length_bytes"], source.len());
    for (file, info) in provenance["artifacts"].as_object().unwrap() {
        let bytes = std::fs::read(fixture(file)).unwrap();
        assert_eq!(info["path"], file.as_str());
        assert_eq!(info["length_bytes"], bytes.len(), "{file}");
        assert_eq!(info["sha256"], digest(&bytes), "{file}");
    }
    assert_eq!(read("case.json")["case_id"], "F-11");
    assert_eq!(read("case.json")["contract_version"], "logical-v1");
    independently_check(
        &source,
        &read("expected-archive.json"),
        &read("expected-visible.json"),
        &read("expected-plan.json"),
        &read("expected-report.json"),
    )
    .unwrap();
}

#[test]
fn f11_independent_checker_rejects_end_zero_and_range_mutations() {
    let source = std::fs::read(fixture("source.bin")).unwrap();
    let archive = read("expected-archive.json");
    let visible = read("expected-visible.json");
    let plan = read("expected-plan.json");
    let report = read("expected-report.json");
    let text = String::from_utf8(source.clone()).unwrap();
    let mut archive_mutations = Vec::new();
    let mut bad_archive = archive.clone();
    bad_archive["records"][0]["cues"][1]["end_usec"] = json!(26000000);
    archive_mutations.push(bad_archive);
    let mut bad_visible = visible.clone();
    bad_visible["cues"][0]["end_usec"] = json!(10000000);
    assert!(independently_check(&source, &archive, &bad_visible, &plan, &report).is_err());
    for mutated in archive_mutations {
        assert!(independently_check(&source, &mutated, &visible, &plan, &report).is_err());
    }
    for changed in [
        text.replacen(
            "\"start_usec\":10000000",
            "\"start_usec\":10000000,\"end_usec\":10000000",
            1,
        ),
        text.replacen("\"start_usec\":0", "\"start_usec\":null", 1),
        text.replacen("\"end_usec\":25000000", "\"end_usec\":null", 1),
        text.replacen("\"end_usec\":25000000", "\"end_usec\":26000000", 1),
    ] {
        let mutated_source = changed.as_bytes();
        let mut consistent_archive = archive.clone();
        let parsed: Value = serde_json::from_slice(mutated_source).unwrap();
        consistent_archive["source_cues"] = parsed["cues"].clone();
        if let Some(records) = consistent_archive["records"].as_array_mut() {
            records[0]["cues"] = parsed["cues"].clone();
        }
        assert!(
            independently_check(
                mutated_source,
                &consistent_archive,
                &visible,
                &plan,
                &report
            )
            .is_err()
        );
    }
}
