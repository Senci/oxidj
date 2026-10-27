use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f08b-logical")
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

// Frozen literal oracle; intentionally independent of production adapter output.
fn check(
    source_bytes: &[u8],
    archive: &Value,
    visible: &Value,
    plan: &Value,
    report: &Value,
) -> Result<(), &'static str> {
    let source: Value = serde_json::from_slice(source_bytes).map_err(|_| "invalid source")?;
    let mut edited = source.clone();
    edited["inUsec"] = json!(11_000_000);
    let operation = json!({"actor":"USER","cause_id":"e1","field":"position","new_usec":11_000_000,"old_usec":10_000_000,"revision":1,"unit_id":"r1"});
    if source["inUsec"] != 10_000_000 {
        return Err("source position mismatch");
    }
    if archive
        != &json!({"source_bytes_hex":hex(source_bytes),"source_object":source,"effective_object":edited,"operations":[operation],"revision":1,"track_id":"t1"})
    {
        return Err("archive mismatch");
    }
    let expected_report = json!([{"code":"UNSUPPORTED_POSITION_EDIT","unit_id":"r1","scope":"root","field":"position","cause_id":"e1","details":{"old_usec":10_000_000,"new_usec":11_000_000}}]);
    if visible != &json!({"cues":[]}) || report != &expected_report {
        return Err("visible/report mismatch");
    }
    if plan
        != &json!({"contract_version":"logical-v1","case_id":"F-08b","target_format":"onelibrary-synthetic","export_policy":"safe-v1","decision_phase":"pre-write","outcome":"WITHHOLD_EXPORT","selected_units":[],"withheld_units":["r1"],"report":expected_report,"target_write":"not-performed","no_write_expected":true})
    {
        return Err("plan mismatch");
    }
    Ok(())
}
fn check_provenance(provenance: &Value, source: &[u8]) -> Result<(), &'static str> {
    if provenance["case_id"] != "F-08b"
        || provenance["classification"] != "logical-only-not-A3-evidence"
        || provenance["source_citation"] != "docs/adr/005-relationship-fixtures.md §3 F-08b"
        || provenance["source_version"]
            != "Synthetic F-05 logical object with F-08 alternate-encoding values; not OneLibrary bytes"
        || provenance["source_length_bytes"] != source.len()
        || provenance["source_sha256"] != digest(source)
    {
        return Err("provenance source claims mismatch");
    }
    let expected = [
        "case.json",
        "expected-archive.json",
        "expected-plan.json",
        "expected-report.json",
        "expected-visible.json",
        "operations.json",
        "source.bin",
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
fn f08b_artifacts_and_provenance_are_frozen() {
    let source = std::fs::read(fixture("source.bin")).unwrap();
    let provenance = read("provenance.json");
    check_provenance(&provenance, &source).unwrap();
    assert_eq!(read("case.json")["case_id"], "F-08b");
    assert_eq!(
        read("operations.json"),
        json!([{"actor":"USER","cause_id":"e1","field":"position","new_usec":11_000_000,"old_usec":10_000_000,"revision":1,"unit_id":"r1"}])
    );
    check(
        &source,
        &read("expected-archive.json"),
        &read("expected-visible.json"),
        &read("expected-plan.json"),
        &read("expected-report.json"),
    )
    .unwrap();
}

#[test]
fn f08b_checker_rejects_report_visibility_and_original_mutations() {
    let source = std::fs::read(fixture("source.bin")).unwrap();
    let archive = read("expected-archive.json");
    let visible = read("expected-visible.json");
    let plan = read("expected-plan.json");
    let report = read("expected-report.json");
    let mut changed_report = report.clone();
    changed_report[0]["details"]["new_usec"] = json!(12_000_000);
    assert!(check(&source, &archive, &visible, &plan, &changed_report).is_err());
    let mut exposed = visible.clone();
    exposed["cues"] = json!([archive["effective_object"].clone()]);
    assert!(check(&source, &archive, &exposed, &plan, &report).is_err());
    let mut altered_archive = archive.clone();
    altered_archive["source_bytes_hex"] = json!("00");
    assert!(check(&source, &altered_archive, &visible, &plan, &report).is_err());
    let mut altered_source: Value = serde_json::from_slice(&source).unwrap();
    altered_source["in150FramePerSec"] = json!(1);
    assert!(
        check(
            &serde_json::to_vec(&altered_source).unwrap(),
            &archive,
            &visible,
            &plan,
            &report
        )
        .is_err()
    );
    let mut changed_provenance = read("provenance.json");
    changed_provenance["artifacts"]["source.bin"]["sha256"] = json!("00");
    assert!(check_provenance(&changed_provenance, &source).is_err());
}
