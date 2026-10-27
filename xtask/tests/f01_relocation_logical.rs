use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f01-relocation-logical")
        .join(name)
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(fixture(name)).unwrap()).unwrap()
}
fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn check(
    s: &Value,
    a: &Value,
    v: &Value,
    p: &Value,
    r: &Value,
    re: &Value,
) -> Result<(), &'static str> {
    let content = "oxidj-f01-content-v1\n";
    if s["case_id"] != "F-01-relocation-success"
        || s["source_kind"] != "mixxx-library-logical"
        || s["content_witness_utf8"] != content
        || s["records"].as_array().is_none_or(|x| x.len() != 2)
    {
        return Err("source mismatch");
    }
    let rs = &s["records"];
    if rs[0]["record_id"] != "r1"
        || rs[0]["path"] != "/music/track1.flac"
        || rs[1]["record_id"] != "r2"
        || rs[1]["path"] != "/music/relocated/track1.flac"
        || rs[0]["external_id"] != "100"
        || rs[1]["external_id"] != "100"
        || rs[0]["grid"] != rs[1]["grid"]
        || rs[0]["cue"] != rs[1]["cue"]
    {
        return Err("A/B witness mismatch");
    }
    let ops = json!([{"operation_id":"reconcile-b","operation":"reconcile","expected_revision":0,"record_id":"r2","source_revision":"s1","track_id":"t1","evidence":{"content_witness_equal":true,"user_confirmed":true},"result":"accepted","revision_after":1},{"operation_id":"select-b","operation":"select-source","expected_revision":1,"record_id":"r2","source_revision":"s1","field":"current_location","track_id":"t1","result":"accepted","revision_after":2}]);
    if a["track_id"] != "t1"
        || a["revision"] != 2
        || a["source_records"].as_array().is_none_or(|x| x.len() != 2)
        || a["source_records"][0]["record_id"] != "r1"
        || a["source_records"][1]["record_id"] != "r2"
        || a["source_records"][0]["content_witness_utf8"] != content
        || a["source_records"][1]["content_witness_utf8"] != content
        || a["operations"] != ops
        || a["current_location_record_id"] != "r2"
        || a["effective_grid"] != rs[1]["grid"]
        || a["effective_cue"] != rs[1]["cue"]
        || a["user_overlays"] != json!([])
    {
        return Err("archive/history mismatch");
    }
    if v["track"]["path"] != "/music/relocated/track1.flac"
        || v["track"]["grid"] != rs[0]["grid"]
        || v["track"]["cue"] != rs[0]["cue"]
        || v["report"] != json!([])
        || r != &json!([])
    {
        return Err("visible/report mismatch");
    }
    if p["outcome"] != "ALLOW_EXPORT"
        || p["export_policy"] != "safe-v1"
        || p["selected_units"] != json!(["t1", "r2", "grid", "c0"])
        || p["withheld_units"] != json!([])
        || p["report"] != json!([])
        || p["no_write_expected"] != false
    {
        return Err("plan mismatch");
    }
    if re["track_count"] != 1
        || re["tracks"].as_array().is_none_or(|x| x.len() != 1)
        || re["tracks"][0]["path"] != "/music/relocated/track1.flac"
        || re["tracks"][0]["grid"] != rs[0]["grid"]
        || re["tracks"][0]["cue"] != rs[0]["cue"]
        || re["report"] != json!([])
        || re["history_present"] != false
        || re["source_record_ids_present"] != false
    {
        return Err("vendor-only re-import mismatch");
    }
    Ok(())
}
#[test]
fn frozen_artifacts_provenance_and_independent_checker() {
    let src = std::fs::read(fixture("source.bin")).unwrap();
    let prov = read("provenance.json");
    assert_eq!(prov["source_sha256"], digest(&src));
    assert_eq!(prov["source_length_bytes"], src.len());
    let names = prov["exact_artifact_set"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    let expected = vec![
        "case.json",
        "expected-archive.json",
        "expected-plan.json",
        "expected-report.json",
        "expected-visible.json",
        "operations.json",
        "source.bin",
        "vendor-reimport.json",
    ];
    assert_eq!(names, expected);
    let actual = std::fs::read_dir(fixture("."))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<std::collections::BTreeSet<_>>();
    let expected_files = expected
        .iter()
        .map(|name| (*name).to_owned())
        .chain(std::iter::once("provenance.json".to_owned()))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(actual, expected_files);
    for (n, i) in prov["artifacts"].as_object().unwrap() {
        let b = std::fs::read(fixture(n)).unwrap();
        assert_eq!(i["length_bytes"], b.len());
        assert_eq!(i["sha256"], digest(&b));
    }
    assert_eq!(prov["checker"], "xtask/tests/f01_relocation_logical.rs");
    assert_eq!(prov["checker_version"], "1");
    assert_eq!(
        read("case.json")["classification"],
        "logical-only-not-A2-evidence"
    );
    let s: Value = serde_json::from_slice(&src).unwrap();
    let a = read("expected-archive.json");
    let v = read("expected-visible.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    let re = read("vendor-reimport.json");
    assert_eq!(read("operations.json"), a["operations"]);
    check(&s, &a, &v, &p, &r, &re).unwrap();
}
#[test]
fn mutation_controls_reject_reconciliation_selection_visible_and_reimport_changes() {
    let s = read("source.bin");
    let a = read("expected-archive.json");
    let v = read("expected-visible.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    let re = read("vendor-reimport.json");
    let mut x = a.clone();
    x["revision"] = json!(1);
    assert!(check(&s, &x, &v, &p, &r, &re).is_err());
    let mut x = a.clone();
    x["current_location_record_id"] = json!("r1");
    assert!(check(&s, &x, &v, &p, &r, &re).is_err());
    let mut x = a.clone();
    x["operations"][0]["expected_revision"] = json!(1);
    assert!(check(&s, &x, &v, &p, &r, &re).is_err());
    let mut x = v.clone();
    x["track"]["path"] = json!("/music/track1.flac");
    assert!(check(&s, &a, &x, &p, &r, &re).is_err());
    let mut x = v.clone();
    x["track"]["cue"]["start_usec"] = json!(10000001);
    assert!(check(&s, &a, &x, &p, &r, &re).is_err());
    let mut x = re.clone();
    x["track_count"] = json!(2);
    assert!(check(&s, &a, &v, &p, &r, &x).is_err());
    let mut x = r.clone();
    x.as_array_mut().unwrap().push(json!({"bad":true}));
    assert!(check(&s, &a, &v, &p, &x, &re).is_err());
}
