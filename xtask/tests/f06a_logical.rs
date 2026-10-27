use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::PathBuf;
fn f(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f06a-logical")
        .join(name)
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(f(name)).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn check(
    src: &Value,
    archive: &Value,
    visible: &Value,
    plan: &Value,
    report: &Value,
) -> Result<(), &'static str> {
    let beats = json!([{"id":"b0","frame":0,"enabled":true,"source":"ANALYZER"},{"id":"b1","frame":120000,"enabled":true,"source":"ANALYZER"},{"id":"b2","frame":240000,"enabled":true,"source":"ANALYZER"}]);
    let cue = json!({"id":"c0","kind":"HotCue","index":0,"start_usec":3000000,"label":"Transition","color":[0,255,0],"flags":0});
    if src["beats"] != beats || src["cue"] != cue || src["sample_rate_hz"] != 48000 {
        return Err("source mismatch");
    }
    let op = json!({"operation_id":"e1","record_id":"r-grid","target_id":"b1","expected_revision":0,"old_value":{"id":"b1","frame":120000,"enabled":true,"source":"ANALYZER"},"new_value":null,"result":"accepted"});
    if archive["source_beats"] != beats
        || archive["source_cue"] != cue
        || archive["operations"] != json!([op])
        || archive["revision"] != 1
        || archive["visible_beats"] != json!([beats[0].clone(), beats[2].clone()])
        || archive["effective_cue"] != cue
    {
        return Err("archive mismatch");
    }
    if visible["beats"] != json!([beats[0].clone(), beats[2].clone()])
        || visible["cue"] != cue
        || visible["report"] != json!([])
        || report != &json!([])
    {
        return Err("visible/report mismatch");
    }
    if plan["case_id"] != "F-06a"
        || plan["outcome"] != "ALLOW_EXPORT"
        || plan["selected_units"] != json!(["b0", "b2", "c0"])
        || plan["report"] != json!([])
    {
        return Err("plan mismatch");
    }
    Ok(())
}
fn check_provenance(p: &Value, identity: &Value, live_checker: &[u8]) -> Result<(), &'static str> {
    let expected_identity = json!({"path":"xtask/tests/f06a_logical.rs","version":"f06a-logical-checker-v1","sha256":digest(live_checker)});
    if identity != &expected_identity
        || p["checker_identity"] != *identity
        || p["case_id"] != "F-06a"
        || p["classification"] != "logical-only-not-A2-evidence"
        || p["source_citation"] != "docs/adr/005-relationship-fixtures.md §3 F-06"
    {
        return Err("checker provenance mismatch");
    }
    let expected = [
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
    let actual = std::fs::read_dir(f("."))
        .map_err(|_| "fixture missing")?
        .map(|e| e.map_err(|_| "entry unreadable"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>();
    let expected_files = expected
        .iter()
        .map(|n| n.to_string())
        .chain(["provenance.json".to_owned()])
        .collect::<BTreeSet<_>>();
    if actual != expected_files {
        return Err("live fixture artifact set mismatch");
    }
    for (name, info) in artifacts {
        let bytes = std::fs::read(f(name)).map_err(|_| "artifact missing")?;
        if info["path"] != name.as_str()
            || info["length_bytes"] != bytes.len()
            || info["sha256"] != digest(&bytes)
        {
            return Err("artifact digest mismatch");
        }
    }
    Ok(())
}
#[test]
fn f06a_frozen_logical_fixture_matches_independent_contract() {
    let identity = read("checker-identity.json");
    let live_checker =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f06a_logical.rs"))
            .unwrap();
    check_provenance(&read("provenance.json"), &identity, &live_checker).unwrap();
    let s = read("source.bin");
    let a = read("expected-archive.json");
    let v = read("expected-visible.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    assert_eq!(read("case.json")["case_id"], "F-06a");
    assert_eq!(read("operations.json"), a["operations"]);
    check(&s, &a, &v, &p, &r).unwrap();
}
#[test]
fn f06a_mutations_reject_cue_changes_and_invented_grid_values() {
    let s = read("source.bin");
    let a = read("expected-archive.json");
    let v = read("expected-visible.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    let identity = read("checker-identity.json");
    let live_checker =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f06a_logical.rs"))
            .unwrap();
    let mut moved = v.clone();
    moved["cue"]["start_usec"] = json!(3000001);
    assert!(check(&s, &a, &moved, &p, &r).is_err());
    let mut withheld = v.clone();
    withheld.as_object_mut().unwrap().remove("cue");
    assert!(check(&s, &a, &withheld, &p, &r).is_err());
    let mut invented = s.clone();
    invented["beats"][1]["bpm"] = json!(128);
    assert!(check(&invented, &a, &v, &p, &r).is_err());
    let mut interpolated = v.clone();
    interpolated["beats"][1] =
        json!({"id":"invented","frame":120000,"enabled":true,"source":"ANALYZER"});
    assert!(check(&s, &a, &interpolated, &p, &r).is_err());
    for (key, value) in [
        ("path", json!("xtask/tests/other.rs")),
        ("version", json!("drift")),
        ("sha256", json!("wrong")),
    ] {
        let mut drift = identity.clone();
        drift[key] = value;
        assert!(check_provenance(&read("provenance.json"), &drift, &live_checker).is_err());
    }
    assert!(check_provenance(&read("provenance.json"), &identity, b"mutated checker").is_err());
    let mut bad = read("provenance.json");
    bad["artifacts"]["unexpected.json"] = json!({});
    assert!(check_provenance(&bad, &identity, &live_checker).is_err());
}
