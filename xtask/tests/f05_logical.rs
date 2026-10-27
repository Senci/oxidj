use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f05-logical")
        .join(name)
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(fixture(name)).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// Literal oracle authored from ADR-005, independent of production adapter paths.
fn check(
    source_bytes: &[u8],
    archive: &Value,
    visible: &Value,
    plan: &Value,
    report: &Value,
    real_plan: &Value,
    real_report: &Value,
) -> Result<(), &'static str> {
    let source: Value = serde_json::from_slice(source_bytes).map_err(|_| "invalid source")?;
    let expected = json!({"colorTableIndex":3,"cueComment":"custom","inUsec":10000000,"isActiveLoop":true,"kind":7,"opaque_hex":"00deadbeefff","outUsec":20000000});
    let projection = json!({"kind":"UNKNOWN(7)","cueComment":"custom","colorTableIndex":3,"isActiveLoop":true,"inUsec":{"value":10000000,"unit":"microseconds"},"outUsec":{"value":20000000,"unit":"microseconds"}});
    if source != expected {
        return Err("source recipe mismatch");
    }
    if archive
        != &json!({"track_id":"t1","revision":0,"operations":[],"source_object":expected,"effective_object":expected,"source_bytes_hex":hex(source_bytes),"logical_projection":projection})
    {
        return Err("archive mismatch");
    }
    if visible != &json!({"object":expected,"source_bytes_hex":hex(source_bytes),"report":[]})
        || report != &json!([])
    {
        return Err("synthetic visible/report mismatch");
    }
    if plan
        != &json!({"contract_version":"logical-v1","case_id":"F-05","target_format":"onelibrary-synthetic","export_policy":"safe-v1","decision_phase":"pre-write","outcome":"ALLOW_EXPORT","selected_units":["r1"],"withheld_units":[],"report":[],"target_write":"permitted-but-not-performed-by-fixture-harness","no_write_expected":false})
    {
        return Err("synthetic plan mismatch");
    }
    let blocked = json!([{"code":"UNSUPPORTED_NATIVE_EXPORT","unit_id":"r1","scope":"root","field":"position","cause_id":null,"details":{"reason":"unqualified-wire-format"}}]);
    if real_report != &blocked
        || real_plan
            != &json!({"contract_version":"logical-v1","case_id":"F-05","target_format":"onelibrary","export_policy":"safe-v1","decision_phase":"pre-write","outcome":"WITHHOLD_EXPORT","selected_units":[],"withheld_units":["r1"],"report":blocked,"target_write":"not-performed","no_write_expected":true})
    {
        return Err("real-target blocked plan/report mismatch");
    }
    Ok(())
}
fn check_provenance(p: &Value, source: &[u8]) -> Result<(), &'static str> {
    if p["case_id"] != "F-05"
        || p["classification"] != "logical-only-not-A3-evidence"
        || p["source_citation"] != "docs/adr/005-relationship-fixtures.md §3 F-05"
        || p["source_version"] != "Synthetic unknown-kind logical object; not OneLibrary bytes"
        || p["source_length_bytes"] != source.len()
        || p["source_sha256"] != digest(source)
    {
        return Err("provenance source claims mismatch");
    }
    let expected = [
        "case.json",
        "expected-archive.json",
        "expected-plan.json",
        "expected-real-target-plan.json",
        "expected-real-target-report.json",
        "expected-report.json",
        "expected-visible.json",
        "operations.json",
        "source.bin",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let artifacts = p["artifacts"]
        .as_object()
        .ok_or("provenance artifacts missing")?;
    if artifacts
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected
    {
        return Err("provenance artifact set mismatch");
    }
    for (name, info) in artifacts {
        let bytes = std::fs::read(fixture(name)).map_err(|_| "artifact missing")?;
        if info["path"] != name.as_str()
            || info["length_bytes"] != bytes.len()
            || info["sha256"] != digest(&bytes)
        {
            return Err("artifact provenance mismatch");
        }
    }
    Ok(())
}
#[test]
fn f05_frozen_artifacts_and_separate_target_decisions_match() {
    let source = std::fs::read(fixture("source.bin")).unwrap();
    check_provenance(&read("provenance.json"), &source).unwrap();
    assert_eq!(read("case.json")["case_id"], "F-05");
    assert_eq!(read("operations.json"), json!([]));
    check(
        &source,
        &read("expected-archive.json"),
        &read("expected-visible.json"),
        &read("expected-plan.json"),
        &read("expected-report.json"),
        &read("expected-real-target-plan.json"),
        &read("expected-real-target-report.json"),
    )
    .unwrap();
}
#[test]
fn f05_checker_rejects_loop_inference_raw_mutation_and_missing_block_entry() {
    let source = std::fs::read(fixture("source.bin")).unwrap();
    let mut archive = read("expected-archive.json");
    let visible = read("expected-visible.json");
    let plan = read("expected-plan.json");
    let report = read("expected-report.json");
    let rp = read("expected-real-target-plan.json");
    let rr = read("expected-real-target-report.json");
    archive["logical_projection"]["kind"] = json!("Loop");
    assert!(check(&source, &archive, &visible, &plan, &report, &rp, &rr).is_err());
    let mut altered = read("expected-archive.json");
    altered["source_bytes_hex"] = json!("00");
    assert!(check(&source, &altered, &visible, &plan, &report, &rp, &rr).is_err());
    let mut missing = rr.clone();
    missing.as_array_mut().unwrap().clear();
    assert!(
        check(
            &source,
            &read("expected-archive.json"),
            &visible,
            &plan,
            &report,
            &rp,
            &missing
        )
        .is_err()
    );
    let mut source_obj: Value = serde_json::from_slice(&source).unwrap();
    source_obj["opaque_hex"] = json!("00deadbeee");
    assert!(
        check(
            &serde_json::to_vec(&source_obj).unwrap(),
            &read("expected-archive.json"),
            &visible,
            &plan,
            &report,
            &rp,
            &rr
        )
        .is_err()
    );
    let mut bad = read("provenance.json");
    bad["artifacts"]["source.bin"]["sha256"] = json!("00");
    assert!(check_provenance(&bad, &source).is_err());
}
