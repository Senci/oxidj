use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/f01-metadata-conflict-logical")
}
fn read(n: &str) -> Value {
    serde_json::from_slice(&std::fs::read(dir().join(n)).unwrap()).unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn checker_identity_matches(identity: &Value, live_source: &[u8]) -> bool {
    identity
        == &json!({"path":"xtask/tests/f01_metadata_conflict_logical.rs","version":"f01-metadata-conflict-logical-v1","sha256":hash(live_source)})
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
    let artifact_keys = p["artifacts"]
        .as_object()
        .ok_or("provenance artifacts missing")?
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    if artifact_keys != names.iter().copied().collect() {
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
    v: &Value,
    p: &Value,
    r: &Value,
) -> Result<(), &'static str> {
    let content = "oxidj-f01-content-v1\n";
    let cue = json!({"id":"c0","index":0,"kind":"HotCue","label":"Intro","start_usec":10000000});
    let agrid = json!({"bpm":"128","anchor_usec":500000});
    let bgrid = json!({"bpm":"129","anchor_usec":500000});
    let records = json!([
 {"record_id":"r1","path":"/music/track1.flac","external_id":"100","source_revision":"s1","grid":agrid,"cue":cue,"content_witness_utf8":content},
 {"record_id":"r2","path":"/music/relocated/track1.flac","external_id":"100","source_revision":"s1","grid":bgrid,"cue":cue,"content_witness_utf8":content}]);
    if s != &json!({"case_id":"F-01-metadata-source-disagreement","source_kind":"mixxx-library-logical","source_version":"logical-v1","content_witness_utf8":content,"records":records})
    {
        return Err("source assertions mismatch");
    }
    let ops = json!([{"operation_id":"reconcile-b","operation":"reconcile","expected_revision":0,"record_id":"r2","source_revision":"s1","track_id":"t1","evidence":{"content_witness_equal":true,"user_confirmed":true},"result":"accepted","revision_after":1},{"operation_id":"select-b","operation":"select-source","expected_revision":1,"record_id":"r2","source_revision":"s1","field":"current_location","track_id":"t1","result":"accepted","revision_after":2}]);
    if o != &ops {
        return Err("operations mismatch");
    }
    let blocked = json!([{"code":"IDENTITY_CONFLICT","record_id":"r2","element_id":null,"field":"identity","cause_operation_id":null,"action":"BLOCK_EXPORT","details":{"reason":"metadata-source-disagreement"}}]);
    let archive = json!({"track_id":"t1","revision":2,"source_records":records,"candidate_records":["r1","r2"],"reconciled_records":["r1","r2"],"operations":ops,"current_location_record_id":"r2","effective_grid":bgrid,"effective_cue":cue,"user_overlays":[],"preselection":{"effective_grid":agrid,"export_outcome":"BLOCK_EXPORT","report":blocked}});
    if a != &archive {
        return Err("archive/history/effective BPM mismatch");
    }
    let report = json!([{"code":"IDENTITY_CONFLICT","record_id":"r2","element_id":null,"field":"identity","cause_operation_id":null,"action":"BLOCK_EXPORT","details":{"reason":"metadata-source-disagreement"}}]);
    if v != &Value::Null
        || p != &json!({"outcome":"BLOCK_EXPORT","export_policy":"safe-v1","selected_units":[],"withheld_units":["r2"],"report":report,"no_write_expected":true})
        || r != &report
    {
        return Err("visible/plan/report mismatch");
    }
    Ok(())
}
#[test]
fn frozen_artifacts_and_literal_checker() {
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
    let prov = read("provenance.json");
    let identity: Value =
        serde_json::from_slice(&std::fs::read(dir().join("checker-identity.json")).unwrap())
            .unwrap();
    let live_checker = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f01_metadata_conflict_logical.rs"),
    )
    .unwrap();
    check_provenance(&prov, &identity, &live_checker).unwrap();
    assert_eq!(
        read("case.json"),
        json!({"case_id":"F-01-metadata-source-disagreement","classification":"logical-only-not-A2-evidence","contract_version":"logical-v1","export_policy":"safe-v1","ordered_element_ids":["c0"],"ordered_record_ids":["r1","r2"],"source_kind":"mixxx-library-logical","source_version":"logical-v1","target_format":"mixxx-library-logical"})
    );
    assert_eq!(read("expected-visible.json"), Value::Null);
    let s: Value = serde_json::from_slice(&src).unwrap();
    let a = read("expected-archive.json");
    let o = read("operations.json");
    let v = read("expected-visible.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    check(&s, &a, &o, &v, &p, &r).unwrap();
}
#[test]
fn mutation_controls_reject_dropped_assertion_wrong_bpm_unblocked_export_and_report_drift() {
    let s = read("source.bin");
    let a = read("expected-archive.json");
    let o = read("operations.json");
    let v = read("expected-visible.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    let mut x = a.clone();
    x["source_records"].as_array_mut().unwrap().pop();
    assert!(check(&s, &x, &o, &v, &p, &r).is_err());
    let mut x = a.clone();
    x["effective_grid"]["bpm"] = json!("128");
    assert!(check(&s, &x, &o, &v, &p, &r).is_err());
    let mut x = p.clone();
    x["outcome"] = json!("ALLOW_EXPORT");
    assert!(check(&s, &a, &o, &v, &x, &r).is_err());
    let mut x = r.clone();
    x[0]["details"]["reason"] = json!("unconfirmed-identity");
    assert!(check(&s, &a, &o, &v, &p, &x).is_err());
    let mut x = p.clone();
    x["report"] = json!([]);
    assert!(check(&s, &a, &o, &v, &p, &x).is_err());
}

#[test]
fn checker_identity_and_artifact_mutations_are_rejected() {
    let identity: Value =
        serde_json::from_slice(&std::fs::read(dir().join("checker-identity.json")).unwrap())
            .unwrap();
    let live = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f01_metadata_conflict_logical.rs"),
    )
    .unwrap();
    let prov = read("provenance.json");
    for (key, value) in [
        ("path", json!("other.rs")),
        ("version", json!("drift")),
        ("sha256", json!("wrong")),
    ] {
        let mut changed = identity.clone();
        changed[key] = value;
        assert!(check_provenance(&prov, &changed, &live).is_err());
    }
    assert!(check_provenance(&prov, &identity, b"mutated checker").is_err());
    let mut changed = prov.clone();
    changed["checker_identity"]["version"] = json!("drift");
    assert!(check_provenance(&changed, &identity, &live).is_err());
    changed = prov.clone();
    changed["artifacts"]
        .as_object_mut()
        .unwrap()
        .remove("checker-identity.json");
    assert!(check_provenance(&changed, &identity, &live).is_err());
    changed = prov.clone();
    changed["artifacts"]["unexpected.json"] = json!({});
    assert!(check_provenance(&changed, &identity, &live).is_err());
    changed = prov;
    changed["exact_artifact_set"] = json!(["drift"]);
    assert!(check_provenance(&changed, &identity, &live).is_err());
}
