use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f04-compose-logical")
        .join(name)
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(path(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
// Independent literal contract: intentionally does not call product import/edit/export code.
fn check(
    src: &[u8],
    ops: &Value,
    archive: &Value,
    visible: &Value,
    plan: &Value,
    report: &Value,
) -> Result<(), &'static str> {
    let source: Value = serde_json::from_slice(src).map_err(|_| "source JSON")?;
    let expected_source = json!({"grid":{"bpm":"128","first_beat_frame":96000,"sample_rate_hz":48000,"source":"ANALYZER"},"cues":[{"color":[255,0,0],"flags":0,"id":"c0","index":0,"kind":"HotCue","label":"Intro","start_usec":10000000}]});
    if source != expected_source {
        return Err("source bytes/value");
    }
    let canonical_source = format!("{}\n", source);
    if src != canonical_source.as_bytes() {
        return Err("source canonical bytes");
    }
    let expected_ops = json!([
      {"operation_id":"e1","target":"r-cues/c0.label","expected_revision":0,"old":"Intro","new":"Opening","result_revision":1},
      {"operation_id":"e2","target":"r-grid/grid.bpm","expected_revision":1,"old":"128","new":"130","result_revision":2}
    ]);
    if ops != &expected_ops {
        return Err("operation order/revisions");
    }
    if archive["revision"] != 2
        || archive["source_grid"] != source["grid"]
        || archive["source_cues"] != source["cues"]
        || archive["operations"] != expected_ops
        || archive["report"] != json!([])
    {
        return Err("archive originals/history");
    }
    let records = archive["records"].as_array().ok_or("records")?;
    if records.len() != 2
        || records[0]["record_id"] != "r-grid"
        || records[1]["record_id"] != "r-cues"
    {
        return Err("record set/order");
    }
    for rec in records {
        if rec["origin"] != "VENDOR:Mixxx"
            || rec["track_id"] != "t1"
            || rec["source_revision"] != "s1"
        {
            return Err("source provenance");
        }
    }
    if records[0]["original_grid"] != source["grid"]
        || records[0]["grid"]
            != json!({"bpm":"130","first_beat_frame":96000,"sample_rate_hz":48000,"source":"ANALYZER"})
        || records[0]["overlays"]
            != json!([{"operation_id":"e2","field":"grid.bpm","value":"130","origin":"USER"}])
    {
        return Err("grid effective/origin");
    }
    if records[1]["original_cues"] != source["cues"]
        || records[1]["cues"]
            != json!([{"color":[255,0,0],"flags":0,"id":"c0","index":0,"kind":"HotCue","label":"Opening","start_usec":10000000}])
        || records[1]["overlays"]
            != json!([{"operation_id":"e1","field":"c0.label","value":"Opening","origin":"USER"}])
    {
        return Err("cue effective/origin");
    }
    if visible
        != &json!({"grid":{"bpm":"130","first_beat_frame":96000,"sample_rate_hz":48000,"source":"ANALYZER"},"cues":[{"color":[255,0,0],"flags":0,"id":"c0","index":0,"kind":"HotCue","label":"Opening","start_usec":10000000}],"report":[]})
        || report != &json!([])
    {
        return Err("visible/report");
    }
    if plan["case_id"] != "F-04-compose"
        || plan["outcome"] != "ALLOW_EXPORT"
        || plan["selected_units"] != json!(["grid", "c0"])
        || plan["report"] != json!([])
    {
        return Err("plan");
    }
    Ok(())
}
#[test]
fn frozen_artifacts_and_literal_contract() {
    let names = [
        "case.json",
        "operations.json",
        "expected-archive.json",
        "expected-visible.json",
        "expected-plan.json",
        "expected-report.json",
        "source.bin",
    ];
    let source = std::fs::read(path("source.bin")).unwrap();
    let prov = read("provenance.json");
    assert_eq!(prov["classification"], "logical-only-not-A2-evidence");
    assert_eq!(prov["source_sha256"], hash(&source));
    assert_eq!(prov["source_length_bytes"], source.len());
    let artifacts = prov["artifacts"].as_object().unwrap();
    assert_eq!(artifacts.len(), names.len());
    let mut actual: Vec<_> = std::fs::read_dir(path("."))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    actual.sort();
    let mut expected: Vec<_> = names.into_iter().map(str::to_owned).collect();
    expected.push("provenance.json".to_owned());
    expected.sort();
    assert_eq!(actual, expected, "fixture artifact set");
    for n in names {
        let b = std::fs::read(path(n)).unwrap();
        let i = &artifacts[n];
        assert_eq!(i["length_bytes"], b.len());
        assert_eq!(i["sha256"], hash(&b));
    }
    assert_eq!(read("case.json")["case_id"], "F-04-compose");
    check(
        &source,
        &read("operations.json"),
        &read("expected-archive.json"),
        &read("expected-visible.json"),
        &read("expected-plan.json"),
        &read("expected-report.json"),
    )
    .unwrap();
}
#[test]
fn mutations_rejected() {
    let source = std::fs::read(path("source.bin")).unwrap();
    let mut ops = read("operations.json");
    let archive = read("expected-archive.json");
    let visible = read("expected-visible.json");
    let plan = read("expected-plan.json");
    let report = read("expected-report.json");
    ops[0]["expected_revision"] = json!(1);
    assert!(check(&source, &ops, &archive, &visible, &plan, &report).is_err());
    let mut ops = read("operations.json");
    ops.as_array_mut().unwrap().swap(0, 1);
    assert!(check(&source, &ops, &archive, &visible, &plan, &report).is_err());
    let mut bad = source.clone();
    *bad.last_mut().unwrap() ^= 1;
    assert!(
        check(
            &bad,
            &read("operations.json"),
            &archive,
            &visible,
            &plan,
            &report
        )
        .is_err()
    );
    for (field, value) in [("label", json!("Intro")), ("start_usec", json!(10000001))] {
        let mut v = visible.clone();
        v["cues"][0][field] = value;
        assert!(
            check(
                &source,
                &read("operations.json"),
                &archive,
                &v,
                &plan,
                &report
            )
            .is_err()
        );
    }
    let mut a = archive.clone();
    a["operations"] = json!([]);
    assert!(
        check(
            &source,
            &read("operations.json"),
            &a,
            &visible,
            &plan,
            &report
        )
        .is_err()
    );
    let mut r = report.clone();
    r.as_array_mut().unwrap().push(json!({"unexpected":true}));
    assert!(
        check(
            &source,
            &read("operations.json"),
            &archive,
            &visible,
            &plan,
            &r
        )
        .is_err()
    );
    let mut a = archive.clone();
    a["records"][1]["overlays"][0]["origin"] = json!("VENDOR:Mixxx");
    assert!(
        check(
            &source,
            &read("operations.json"),
            &a,
            &visible,
            &plan,
            &report
        )
        .is_err()
    );
}
