use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/f01-unconfirmed-logical")
}
fn read(n: &str) -> Value {
    serde_json::from_slice(&std::fs::read(dir().join(n)).unwrap()).unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn checker_identity_matches(identity: &Value, live_checker: &[u8]) -> bool {
    identity
        == &json!({"path":"xtask/tests/f01_unconfirmed_logical.rs","version":"f01-unconfirmed-logical-v1","sha256":hash(live_checker)})
}
fn check_provenance(p: &Value, identity: &Value, live_checker: &[u8]) -> Result<(), &'static str> {
    if !checker_identity_matches(identity, live_checker) || p["checker_identity"] != *identity {
        return Err("checker identity mismatch");
    }
    let names = [
        "case.json",
        "expected-archive.json",
        "expected-plan.json",
        "expected-report.json",
        "expected-visible.json",
        "operations.json",
        "source.bin",
        "checker-identity.json",
    ];
    if p["exact_artifact_set"] != json!(names) {
        return Err("provenance artifact set mismatch");
    }
    let expected = names
        .iter()
        .map(|s| s.to_string())
        .chain(std::iter::once("provenance.json".into()))
        .collect::<std::collections::BTreeSet<_>>();
    let actual = std::fs::read_dir(dir())
        .map_err(|_| "fixture directory missing")?
        .map(|e| e.map_err(|_| "fixture entry unreadable"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect::<std::collections::BTreeSet<_>>();
    if actual != expected {
        return Err("live fixture artifact set mismatch");
    }
    let keys = p["artifacts"]
        .as_object()
        .ok_or("provenance artifacts missing")?
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    if keys != names.iter().copied().collect() {
        return Err("provenance artifact set mismatch");
    }
    for n in names {
        let b = std::fs::read(dir().join(n)).map_err(|_| "artifact missing")?;
        if p["artifacts"][n]["path"] != n
            || p["artifacts"][n]["length_bytes"] != b.len()
            || p["artifacts"][n]["sha256"] != hash(&b)
        {
            return Err("artifact provenance mismatch");
        }
    }
    Ok(())
}
fn check(
    s: &Value,
    a: &Value,
    o: &Value,
    p: &Value,
    r: &Value,
    v: &Value,
) -> Result<(), &'static str> {
    let grid = json!({"bpm":"128","anchor_usec":500000});
    let cue = json!({"id":"c0","index":0,"kind":"HotCue","label":"Intro","start_usec":10000000});
    let witness = "oxidj-f01-content-v1\n";
    let records = json!([
      {"record_id":"r1","namespace":"mixxx-library-A","path":"/music/track1.flac","external_id":"100","source_revision":"s1","grid":grid,"cue":cue,"content_witness_utf8":witness},
      {"record_id":"r2","namespace":"mixxx-library-B","path":"/music/candidate.flac","external_id":"100","source_revision":"s1","grid":grid,"cue":cue,"content_witness_utf8":witness}
    ]);
    if s != &json!({"case_id":"F-01-unconfirmed-cross-namespace-identity","source_kind":"mixxx-library-logical","source_version":"logical-v1","records":records})
    {
        return Err("source/namespace/identity/content mismatch");
    }
    let ops = json!([
      {"operation_id":"import-a","operation":"import","record_id":"r1","expected_revision":0,"result":"accepted","revision_after":1},
      {"operation_id":"import-b","operation":"import-candidate","record_id":"r2","expected_revision":1,"result":"identity-conflict","revision_after":1,"reason":"unconfirmed-identity"}
    ]);
    if o != &ops {
        return Err("operations or confirmation mismatch");
    }
    if a != &json!({"track_id":"t1","revision":1,"source_records":records,"candidate_records":["r1","r2"],"reconciled_records":["r1"],"operations":ops,"current_location_record_id":"r1","effective_grid":grid,"effective_cue":cue,"user_overlays":[]})
    {
        return Err("archive/merge/original mutation mismatch");
    }
    let report = json!([{"code":"IDENTITY_CONFLICT","record_id":"r2","element_id":null,"field":"identity","cause_operation_id":null,"action":"BLOCK_EXPORT","details":{"reason":"unconfirmed-identity"}}]);
    if r != &report
        || p != &json!({"outcome":"BLOCK_EXPORT","export_policy":"safe-v1","selected_units":[],"withheld_units":["r2"],"report":report,"no_write_expected":true})
    {
        return Err("plan/report mismatch");
    }
    if !v.is_null() {
        return Err("blocked export must have null visible output");
    }
    Ok(())
}
#[test]
fn frozen_artifact_set_provenance_and_contract() {
    let names = [
        "case.json",
        "expected-archive.json",
        "expected-plan.json",
        "expected-report.json",
        "expected-visible.json",
        "operations.json",
        "source.bin",
        "checker-identity.json",
    ];
    let prov = read("provenance.json");
    let src = std::fs::read(dir().join("source.bin")).unwrap();
    let identity = read("checker-identity.json");
    let live_checker = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f01_unconfirmed_logical.rs"),
    )
    .unwrap();
    check_provenance(&prov, &identity, &live_checker).unwrap();
    assert_eq!(prov["source_length_bytes"], src.len());
    assert_eq!(prov["source_sha256"], hash(&src));
    assert_eq!(prov["exact_artifact_set"], json!(names));
    let actual = std::fs::read_dir(dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        actual,
        names
            .iter()
            .map(|s| s.to_string())
            .chain(std::iter::once("provenance.json".into()))
            .collect()
    );
    for n in names {
        let b = std::fs::read(dir().join(n)).unwrap();
        assert_eq!(prov["artifacts"][n]["path"], n);
        assert_eq!(prov["artifacts"][n]["length_bytes"], b.len());
        assert_eq!(prov["artifacts"][n]["sha256"], hash(&b));
    }
    let s: Value = serde_json::from_slice(&src).unwrap();
    let a = read("expected-archive.json");
    let o = read("operations.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    let v = read("expected-visible.json");
    assert_eq!(
        read("case.json"),
        json!({"case_id":"F-01-unconfirmed-cross-namespace-identity","classification":"logical-only-not-A2-evidence","contract_version":"logical-v1","export_policy":"safe-v1","ordered_element_ids":["c0"],"ordered_record_ids":["r1","r2"],"source_kind":"mixxx-library-logical","source_version":"logical-v1","target_format":"mixxx-library-logical"})
    );
    assert_eq!(prov["checker"], "xtask/tests/f01_unconfirmed_logical.rs");
    assert_eq!(prov["checker_version"], "1");
    check(&s, &a, &o, &p, &r, &v).unwrap();
    let mut changed_identity = identity.clone();
    changed_identity["version"] = json!("drift");
    assert!(check_provenance(&prov, &changed_identity, &live_checker).is_err());
    assert!(check_provenance(&prov, &identity, b"mutated checker").is_err());
    let mut changed = prov.clone();
    changed["exact_artifact_set"] = json!(["source.bin"]);
    assert!(check_provenance(&changed, &identity, &live_checker).is_err());
}
#[test]
fn literal_mutation_controls_reject_merge_drift_confirmation_and_report_changes() {
    let s = read("source.bin");
    let a = read("expected-archive.json");
    let o = read("operations.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    let v = read("expected-visible.json");
    let mut x = a.clone();
    x["reconciled_records"] = json!(["r1", "r2"]);
    assert!(check(&s, &x, &o, &p, &r, &v).is_err());
    let mut x = s.clone();
    x["records"][0]["namespace"] = json!("mixxx-library-B");
    assert!(check(&x, &a, &o, &p, &r, &v).is_err());
    let mut x = s.clone();
    x["records"][0]["external_id"] = json!("101");
    assert!(check(&x, &a, &o, &p, &r, &v).is_err());
    let mut x = s.clone();
    x["records"][1]["content_witness_utf8"] = json!("drift\n");
    assert!(check(&x, &a, &o, &p, &r, &v).is_err());
    let mut x = o.clone();
    x[1]["operation"] = json!("reconcile");
    x[1]["result"] = json!("accepted");
    assert!(check(&s, &a, &x, &p, &r, &v).is_err());
    let mut x = r.clone();
    x[0]["details"]["reason"] = json!("source-id-content-disagreement");
    assert!(check(&s, &a, &o, &p, &x, &v).is_err());
    let mut x = p.clone();
    x["no_write_expected"] = json!(false);
    assert!(check(&s, &a, &o, &x, &r, &v).is_err());
}
