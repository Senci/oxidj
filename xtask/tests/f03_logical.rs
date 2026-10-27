use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f03-logical")
        .join(name)
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(fixture(name)).unwrap()).unwrap()
}
fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
// Hand-authored DTO contract oracle; deliberately independent of product import/projection code.
fn check(src: &[u8], a: &Value, v: &Value, p: &Value, r: &Value) -> Result<(), &'static str> {
    let s: Value = serde_json::from_slice(src).map_err(|_| "source JSON")?;
    let g = &s["grid"];
    if g != &json!({"bpm":"128","first_beat_frame":96000,"sample_rate_hz":48000,"source":"ANALYZER"})
    {
        return Err("grid mismatch");
    }
    let cues = s["cues"].as_array().ok_or("cues absent")?;
    if cues.len() != 4 {
        return Err("cue count/order");
    }
    let expected = json!([
 {"id":"c0","kind":"HotCue","index":0,"start_usec":10000000,"label":"Intro","color":[255,0,0],"flags":0},
 {"id":"c1","kind":"HotCue","index":1,"start_usec":20000000,"label":"Drop","color":[0,0,255],"flags":0},
 {"id":"c2","kind":"Loop","start_usec":15000000,"end_usec":25000000,"label":"Loop 1","flags":0},
 {"id":"c3","kind":"MainCue","start_usec":5000000,"flags":0}]);
    if json!(cues) != expected {
        return Err("cue DTO mismatch");
    }
    if a["source_grid"] != *g
        || a["source_cues"] != json!(cues)
        || a["revision"] != 0
        || a["operations"] != json!([])
        || a["records"].as_array().map(Vec::len) != Some(2)
    {
        return Err("archive mismatch");
    }
    let rec = a["records"].as_array().unwrap();
    if rec[0]["record_id"] != "r-grid"
        || rec[0]["grid"] != *g
        || rec[1]["record_id"] != "r-cues"
        || rec[1]["cues"] != json!(cues)
    {
        return Err("record projection mismatch");
    }
    if v["grid"] != *g || v["cues"] != json!(cues) || v["report"] != json!([]) || r != &json!([]) {
        return Err("visible/report mismatch");
    }
    if p["case_id"] != "F-03"
        || p["outcome"] != "ALLOW_EXPORT"
        || p["selected_units"] != json!(["grid", "c0", "c1", "c2", "c3"])
        || p["report"] != json!([])
    {
        return Err("plan mismatch");
    }
    Ok(())
}
#[test]
fn f03_artifacts_frozen_and_independently_equal() {
    let source = std::fs::read(fixture("source.bin")).unwrap();
    let provenance = read("provenance.json");
    assert_eq!(provenance["classification"], "logical-only-not-A2-evidence");
    assert_eq!(provenance["source_sha256"], digest(&source));
    assert_eq!(provenance["source_length_bytes"], source.len());
    for (file, info) in provenance["artifacts"].as_object().unwrap() {
        let b = std::fs::read(fixture(file)).unwrap();
        assert_eq!(info["length_bytes"], b.len());
        assert_eq!(info["sha256"], digest(&b));
    }
    assert_eq!(read("case.json")["case_id"], "F-03");
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
fn f03_mutation_controls_reject_index_end_collapse_and_grid() {
    let source = std::fs::read(fixture("source.bin")).unwrap();
    let archive = read("expected-archive.json");
    let visible = read("expected-visible.json");
    let plan = read("expected-plan.json");
    let report = read("expected-report.json");
    let mut cases = Vec::new();
    let mut x: Value = serde_json::from_slice(&source).unwrap();
    x["cues"][0].as_object_mut().unwrap().remove("index");
    cases.push(x);
    let mut x: Value = serde_json::from_slice(&source).unwrap();
    x["cues"][0]["end_usec"] = json!(10000000);
    cases.push(x);
    let mut x: Value = serde_json::from_slice(&source).unwrap();
    x["cues"][2].as_object_mut().unwrap().remove("end_usec");
    cases.push(x);
    let mut x: Value = serde_json::from_slice(&source).unwrap();
    x["grid"]["bpm"] = json!("129");
    cases.push(x);
    for changed in cases {
        let bytes = (serde_json::to_string(&changed).unwrap() + "\n").into_bytes();
        assert!(check(&bytes, &archive, &visible, &plan, &report).is_err());
    }
    let mut bad = visible.clone();
    bad["grid"]["first_beat_frame"] = json!(96001);
    assert!(check(&source, &archive, &bad, &plan, &report).is_err());
}
