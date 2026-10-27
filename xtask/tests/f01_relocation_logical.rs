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
    let grid = json!({"bpm":"128","anchor_usec":500000});
    let cue = json!({"id":"c0","index":0,"kind":"HotCue","label":"Intro","start_usec":10000000});
    let source_records = json!([
        {"record_id":"r1","path":"/music/track1.flac","external_id":"100","source_revision":"s1","grid":grid,"cue":cue,"content_witness_utf8":content},
        {"record_id":"r2","path":"/music/relocated/track1.flac","external_id":"100","source_revision":"s1","grid":grid,"cue":cue,"content_witness_utf8":content}
    ]);
    let source = json!({"case_id":"F-01-relocation-success","source_kind":"mixxx-library-logical","source_version":"logical-v1","content_witness_utf8":content,"records":[
        {"record_id":"r1","path":"/music/track1.flac","external_id":"100","source_revision":"s1","grid":grid,"cue":cue},
        {"record_id":"r2","path":"/music/relocated/track1.flac","external_id":"100","source_revision":"s1","grid":grid,"cue":cue}
    ]});
    if s != &source {
        return Err("source contract mismatch");
    }
    let ops = json!([{"operation_id":"reconcile-b","operation":"reconcile","expected_revision":0,"record_id":"r2","source_revision":"s1","track_id":"t1","evidence":{"content_witness_equal":true,"user_confirmed":true},"result":"accepted","revision_after":1},{"operation_id":"select-b","operation":"select-source","expected_revision":1,"record_id":"r2","source_revision":"s1","field":"current_location","track_id":"t1","result":"accepted","revision_after":2}]);
    let expected_archive = json!({"track_id":"t1","revision":2,"source_records":source_records,"candidate_records":["r1","r2"],"reconciled_records":["r1","r2"],"operations":ops,"current_location_record_id":"r2","effective_grid":grid,"effective_cue":cue,"user_overlays":[]});
    if a != &expected_archive {
        return Err("archive/history mismatch");
    }
    let expected_track =
        json!({"path":"/music/relocated/track1.flac","external_id":"100","grid":grid,"cue":cue});
    if v != &json!({"track":expected_track,"report":[]}) || r != &json!([]) {
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
        || re["tracks"][0]["grid"] != json!({"bpm":"128","anchor_usec":500000})
        || re["tracks"][0]["cue"]
            != json!({"id":"c0","index":0,"kind":"HotCue","label":"Intro","start_usec":10000000})
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
    let expected_case = json!({"case_id":"F-01-relocation-success","classification":"logical-only-not-A2-evidence","contract_version":"logical-v1","export_policy":"safe-v1","ordered_element_ids":["c0"],"ordered_record_ids":["r1","r2"],"source_kind":"mixxx-library-logical","source_version":"logical-v1","target_format":"mixxx-library-logical"});
    let expected_names = vec![
        "case.json",
        "expected-archive.json",
        "expected-plan.json",
        "expected-report.json",
        "expected-visible.json",
        "operations.json",
        "source.bin",
        "vendor-reimport.json",
    ];
    let expected_prov = json!({"case_id":"F-01-relocation-success","checker":"xtask/tests/f01_relocation_logical.rs","checker_version":"1","classification":"logical-only-not-A2-evidence","source_citation":"docs/adr/005-relationship-fixtures.md §3 F-01","source_version":"Mixxx 2.5.6 logical recipe; synthetic evidence, not Mixxx transport bytes or audio","exact_artifact_set":expected_names,"source_length_bytes":src.len(),"source_sha256":digest(&src)});
    let mut expected_metadata = expected_prov.as_object().unwrap().clone();
    expected_metadata.insert("artifacts".into(), prov["artifacts"].clone());
    assert!(
        !(prov
            .as_object()
            .unwrap()
            .keys()
            .any(|k| !expected_metadata.contains_key(k))
            || prov != Value::Object(expected_metadata)),
        "provenance metadata contract mismatch"
    );
    assert_eq!(read("case.json"), expected_case, "case metadata mismatch");
    assert_eq!(
        prov["artifacts"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>(),
        expected_names.iter().map(|n| (*n).to_owned()).collect()
    );
    for n in &expected_names {
        let b = std::fs::read(fixture(n)).unwrap();
        let i = &prov["artifacts"][*n];
        assert_eq!(i["path"], *n);
        assert_eq!(i["length_bytes"], b.len());
        assert_eq!(i["sha256"], digest(&b));
    }
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
    let mut x = s.clone();
    x["records"][0]["grid"]["bpm"] = json!(127);
    assert!(check(&x, &a, &v, &p, &r, &re).is_err());
    let mut x = a.clone();
    x["source_records"].as_array_mut().unwrap().reverse();
    assert!(check(&s, &x, &v, &p, &r, &re).is_err());
    let mut x = a.clone();
    x["candidate_records"] = json!(["r1"]);
    assert!(check(&s, &x, &v, &p, &r, &re).is_err());
    let mut x = a.clone();
    x["reconciled_records"] = json!([]);
    assert!(check(&s, &x, &v, &p, &r, &re).is_err());
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
