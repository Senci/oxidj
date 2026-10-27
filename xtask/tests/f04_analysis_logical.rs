use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
fn path(n: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f04-analysis-logical")
        .join(n)
}
fn read(n: &str) -> Value {
    serde_json::from_slice(&std::fs::read(path(n)).unwrap()).unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn provenance_matches(prov: &Value, expected: &Value) -> bool {
    prov == expected
}
fn checker_identity() -> Value {
    serde_json::from_slice(&std::fs::read(path("checker-identity.json")).unwrap()).unwrap()
}
fn checker_identity_matches(identity: &Value, live_source: &[u8]) -> bool {
    identity
        == &json!({
            "path": "xtask/tests/f04_analysis_logical.rs",
            "version": "f04-analysis-precedence-checker-v1",
            "sha256": hash(live_source)
        })
}
fn check(s: &Value, o: &Value, a: &Value, mid: &Value, v: &Value, p: &Value, r: &Value) -> bool {
    let grid =
        json!({"bpm":"128","first_beat_frame":96000,"sample_rate_hz":48000,"source":"ANALYZER"});
    let cues = json!([{"color":[255,0,0],"flags":0,"id":"c0","index":0,"kind":"HotCue","label":"Intro","start_usec":10000000}]);
    let expected = json!([
 {"operation_id":"e1","target":"r-cues/c0.label","expected_revision":0,"old":"Intro","new":"Opening","result_revision":1},
 {"operation_id":"e2","target":"r-grid/grid.bpm","expected_revision":1,"old":"128","new":"130","result_revision":2},
 {"operation_id":"e3","target":"r-cues/c0.label","expected_revision":2,"old":"Opening","new":"Opening 2","result_revision":3},
 {"operation_id":"a1","target":"r-grid/grid.bpm","expected_revision":3,"analyzer":"m1-synthetic","analyzer_version":"1","sequence":1,"bpm":"129","result":"Appended","result_revision":4},
 {"operation_id":"clear-user","target":"r-grid/grid.bpm","expected_revision":4,"result":"Cleared","result_revision":5}]);
    let after =
        json!({"bpm":"129","first_beat_frame":96000,"sample_rate_hz":48000,"source":"ANALYZER"});
    s == &json!({"grid":grid,"cues":cues})
        && o == &expected
        && a["revision"] == 5
        && a["source_grid"] == s["grid"]
        && a["source_cues"] == s["cues"]
        && a["operations"] == expected
        && a["records"][0]["original_grid"] == grid
        && a["records"][0]["grid"] == after
        && a["records"][0]["analysis"]
            == json!([{"operation_id":"a1","analyzer":"m1-synthetic","analyzer_version":"1","sequence":1,"source_revision":"s1","content_revision":3,"bpm":"129"}])
        && a["records"][0]["user_history"]
            == json!([{"from":"128","operation_id":"e2","revision":1,"to":"130"},{"cleared":"130","operation_id":"clear-user","revision":4}])
        && mid
            == &json!({"revision":4,"grid":{"bpm":"130","first_beat_frame":96000,"sample_rate_hz":48000,"source":"ANALYZER"},"analysis":[{"operation_id":"a1","analyzer":"m1-synthetic","analyzer_version":"1","sequence":1,"source_revision":"s1","content_revision":3,"bpm":"129"}],"user_history":[{"from":"128","operation_id":"e2","revision":1,"to":"130"}],"overlays":[{"field":"grid.bpm","operation_id":"e2","origin":"USER","value":"130"}],"report":[]})
        && a["records"][1]["original_cues"] == cues
        && v == &json!({"grid":after,"cues":[{"color":[255,0,0],"flags":0,"id":"c0","index":0,"kind":"HotCue","label":"Opening 2","start_usec":10000000}],"report":[]})
        && a["report"] == json!([])
        && r == &json!([])
        && p == &json!({"case_id":"F-04-analysis-precedence","outcome":"ALLOW_EXPORT","selected_units":["grid","c0"],"report":[]})
}
#[test]
fn frozen_artifacts_and_literal_state() {
    let names = [
        "case.json",
        "operations.json",
        "expected-archive.json",
        "expected-intermediate.json",
        "expected-visible.json",
        "expected-plan.json",
        "expected-report.json",
        "source.bin",
        "checker-identity.json",
    ];
    let b = std::fs::read(path("source.bin")).unwrap();
    let s: Value = serde_json::from_slice(&b).unwrap();
    assert_eq!(b, format!("{}\n", s).as_bytes());
    let prov = read("provenance.json");
    let identity = checker_identity();
    let checker_source = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f04_analysis_logical.rs"),
    )
    .unwrap();
    assert!(checker_identity_matches(&identity, &checker_source));
    let mut artifact_map = serde_json::Map::new();
    for n in names {
        let x = std::fs::read(path(n)).unwrap();
        artifact_map.insert(
            n.to_string(),
            json!({"length_bytes":x.len(),"path":n,"sha256":hash(&x)}),
        );
    }
    let expected_provenance = json!({
        "artifacts": artifact_map,
        "checker": "independent-literal-v1",
        "checker_identity": identity,
        "classification": "logical-only-not-A2-evidence",
        "source_length_bytes": b.len(),
        "source_sha256": hash(&b),
        "source_version": "logical-v1"
    });
    assert!(provenance_matches(&prov, &expected_provenance));
    let mut actual: Vec<_> = std::fs::read_dir(path("."))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    actual.sort();
    let mut expected: Vec<_> = names
        .iter()
        .map(|x| x.to_string())
        .chain(["provenance.json".into()])
        .collect();
    expected.sort();
    assert_eq!(actual, expected);
    for n in names {
        let x = std::fs::read(path(n)).unwrap();
        assert_eq!(prov["artifacts"][n]["length_bytes"], x.len());
        assert_eq!(prov["artifacts"][n]["sha256"], hash(&x));
    }
    assert!(check(
        &s,
        &read("operations.json"),
        &read("expected-archive.json"),
        &read("expected-intermediate.json"),
        &read("expected-visible.json"),
        &read("expected-plan.json"),
        &read("expected-report.json")
    ));
}
#[test]
fn mutation_controls_reject_drift() {
    let s: Value = serde_json::from_slice(&std::fs::read(path("source.bin")).unwrap()).unwrap();
    let o = read("operations.json");
    let a = read("expected-archive.json");
    let mid = read("expected-intermediate.json");
    let v = read("expected-visible.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    for (idx, key, val) in [
        (3, "analyzer", json!("wrong")),
        (3, "analyzer_version", json!("2")),
        (3, "expected_revision", json!(2)),
        (3, "sequence", json!(2)),
        (3, "bpm", json!("128")),
        (4, "result_revision", json!(4)),
    ] {
        let mut m = o.clone();
        m[idx][key] = val;
        assert!(!check(&s, &m, &a, &mid, &v, &p, &r));
    }
    let mut m = a.clone();
    m["records"][0]["grid"]["bpm"] = json!("130");
    assert!(!check(&s, &o, &m, &mid, &v, &p, &r));
    let mut m = a.clone();
    m["records"][0]["analysis"][0]["source_revision"] = json!("s2");
    assert!(!check(&s, &o, &m, &mid, &v, &p, &r));
    let mut m = a.clone();
    m["records"][0]["analysis"][0]["content_revision"] = json!(2);
    assert!(!check(&s, &o, &m, &mid, &v, &p, &r));
    let mut m = a.clone();
    m["records"][0]["user_history"] = json!([]);
    assert!(!check(&s, &o, &m, &mid, &v, &p, &r));
    let mut m = v.clone();
    m["grid"]["bpm"] = json!("130");
    assert!(!check(&s, &o, &a, &mid, &m, &p, &r));
    let mut m = r.clone();
    m.as_array_mut().unwrap().push(json!({"drift":true}));
    assert!(!check(&s, &o, &a, &mid, &v, &p, &m));
    let mut m = p.clone();
    m["report"] = json!([{"drift":true}]);
    assert!(!check(&s, &o, &a, &mid, &v, &m, &r));
    let mut m = mid.clone();
    m["grid"]["bpm"] = json!("129");
    assert!(!check(&s, &o, &a, &m, &v, &p, &r));

    let live = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f04_analysis_logical.rs"),
    )
    .unwrap();
    let identity = checker_identity();
    for (key, value) in [
        ("path", json!("xtask/tests/other.rs")),
        ("version", json!("f04-analysis-precedence-checker-v2")),
        ("sha256", json!("wrong-digest")),
    ] {
        let mut drift = identity.clone();
        drift[key] = value;
        assert!(!checker_identity_matches(&drift, &live));
    }
    assert!(!checker_identity_matches(
        &identity,
        b"mutated checker source"
    ));

    let mut provenance = read("provenance.json");
    let mut expected = provenance.clone();
    provenance["checker"] = json!("drifted-checker");
    assert!(!provenance_matches(&provenance, &expected));
    provenance = read("provenance.json");
    expected = provenance.clone();
    provenance["artifacts"]["unexpected.json"] =
        json!({"path":"unexpected.json","length_bytes":0,"sha256":""});
    assert!(!provenance_matches(&provenance, &expected));
}
