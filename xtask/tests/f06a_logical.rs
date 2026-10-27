use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2 as _;
use std::path::PathBuf;
fn f(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f06a-logical")
        .join(name)
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(f(name)).unwrap()).unwrap()
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
#[test]
fn f06a_frozen_logical_fixture_matches_independent_contract() {
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
}
