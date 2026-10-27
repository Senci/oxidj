use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
fn path(n: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f04-replay-logical")
        .join(n)
}
fn read(n: &str) -> Value {
    serde_json::from_slice(&std::fs::read(path(n)).unwrap()).unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn checker_identity() -> Value {
    serde_json::from_slice(&std::fs::read(path("checker-identity.json")).unwrap()).unwrap()
}
fn identity_matches(identity: &Value, source: &[u8]) -> bool {
    identity
        == &json!({"path":"xtask/tests/f04_replay_logical.rs","version":"f04-replay-checker-v1","sha256":hash(source)})
}
fn provenance_valid(prov: &Value, names: &[&str]) -> bool {
    let Ok(identity_bytes) = std::fs::read(path("checker-identity.json")) else {
        return false;
    };
    let Ok(identity) = serde_json::from_slice::<Value>(&identity_bytes) else {
        return false;
    };
    let Ok(source) = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f04_replay_logical.rs"),
    ) else {
        return false;
    };
    if !identity_matches(&identity, &source)
        || prov["checker_identity"] != identity
        || prov["classification"] != "logical-only-not-A2-evidence"
        || prov["checker"] != "independent-literal-v1"
    {
        return false;
    }
    let Some(artifacts) = prov["artifacts"].as_object() else {
        return false;
    };
    artifacts.len() == names.len()
        && names.iter().all(|name| {
            let Ok(bytes) = std::fs::read(path(name)) else {
                return false;
            };
            artifacts[*name]["path"] == *name
                && artifacts[*name]["length_bytes"] == bytes.len()
                && artifacts[*name]["sha256"] == hash(&bytes)
        })
}
fn ops() -> Value {
    json!([
     {"operation_id":"e1","target":"r-cues/c0.label","expected_revision":0,"old":"Intro","new":"Opening","result_revision":1},
     {"operation_id":"e2","target":"r-grid/grid.bpm","expected_revision":1,"old":"128","new":"130","result_revision":2},
     {"operation_id":"e3","target":"r-cues/c0.label","expected_revision":2,"old":"Opening","new":"Opening 2","result_revision":3},
     {"operation_id":"e4","target":"r-cues/c0.label","expected_revision":0,"old":"Intro","new":"Wrong","result":"Conflict","result_revision":3},
     {"operation_id":"e3","replay":"identical","result":"NoChange","result_revision":3},
     {"operation_id":"e3","target":"r-cues/c0.label","expected_revision":3,"old":"Opening 2","new":"Different","result":"IdReuseError","result_revision":3}
    ])
}
// Literal independent checker; it does not invoke importer/editor/exporter code.
fn check(
    source: &Value,
    operations: &Value,
    archive: &Value,
    visible: &Value,
    plan: &Value,
    report: &Value,
) -> bool {
    let expected_source = json!({"grid":{"bpm":"128","first_beat_frame":96000,"sample_rate_hz":48000,"source":"ANALYZER"},"cues":[{"color":[255,0,0],"flags":0,"id":"c0","index":0,"kind":"HotCue","label":"Intro","start_usec":10000000}]});
    let final_cue = json!([{"color":[255,0,0],"flags":0,"id":"c0","index":0,"kind":"HotCue","label":"Opening 2","start_usec":10000000}]);
    let grid =
        json!({"bpm":"130","first_beat_frame":96000,"sample_rate_hz":48000,"source":"ANALYZER"});
    if source != &expected_source
        || operations != &ops()
        || archive["revision"] != 3
        || archive["operations"] != ops()
        || archive["source_grid"] != source["grid"]
        || archive["source_cues"] != source["cues"]
        || archive["report"] != json!([])
    {
        return false;
    }
    let rs = match archive["records"].as_array() {
        Some(x) => x,
        None => return false,
    };
    if rs.len() != 2
        || rs[0]["record_id"] != "r-grid"
        || rs[0]["original_grid"] != source["grid"]
        || rs[0]["grid"] != grid
        || rs[0]["overlays"]
            != json!([{"operation_id":"e2","field":"grid.bpm","value":"130","origin":"USER"}])
        || rs[1]["record_id"] != "r-cues"
        || rs[1]["original_cues"] != source["cues"]
        || rs[1]["cues"] != final_cue
        || rs[1]["overlays"]
            != json!([{"operation_id":"e3","field":"c0.label","value":"Opening 2","origin":"USER"}])
    {
        return false;
    }
    for r in rs {
        if r["origin"] != "VENDOR:Mixxx" || r["track_id"] != "t1" || r["source_revision"] != "s1" {
            return false;
        }
    }
    visible == &json!({"grid":grid,"cues":final_cue,"report":[]})
        && report == &json!([])
        && plan
            == &json!({"case_id":"F-04-stale-replay","outcome":"ALLOW_EXPORT","selected_units":["grid","c0"],"report":[]})
}
#[test]
fn frozen_contract_and_artifacts() {
    let names = [
        "case.json",
        "operations.json",
        "expected-archive.json",
        "expected-visible.json",
        "expected-plan.json",
        "expected-report.json",
        "source.bin",
        "checker-identity.json",
    ];
    let source_bytes = std::fs::read(path("source.bin")).unwrap();
    let source: Value = serde_json::from_slice(&source_bytes).unwrap();
    assert_eq!(source_bytes, format!("{}\n", source).as_bytes());
    let prov = read("provenance.json");
    assert_eq!(prov["source_version"], "logical-v1");
    let identity = checker_identity();
    let live_source = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f04_replay_logical.rs"),
    )
    .unwrap();
    assert!(identity_matches(&identity, &live_source));
    for (key, value) in [
        ("path", json!("xtask/tests/other.rs")),
        ("version", json!("f04-replay-checker-v2")),
        ("sha256", json!("wrong-digest")),
    ] {
        let mut drift = identity.clone();
        drift[key] = value;
        assert!(!identity_matches(&drift, &live_source));
    }
    assert!(!identity_matches(&identity, b"mutated checker source"));
    assert!(provenance_valid(&prov, &names));
    assert_eq!(prov["source_sha256"], hash(&source_bytes));
    assert_eq!(prov["source_length_bytes"], source_bytes.len());
    let a = prov["artifacts"].as_object().unwrap();
    assert_eq!(a.len(), names.len());
    let mut actual: Vec<_> = std::fs::read_dir(path("."))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    actual.sort();
    let mut expected: Vec<_> = names
        .iter()
        .map(|x| x.to_string())
        .chain(["provenance.json".to_owned()])
        .collect();
    expected.sort();
    assert_eq!(actual, expected);
    let mut drift = prov.clone();
    drift["checker_identity"]["version"] = json!("drift");
    assert!(!provenance_valid(&drift, &names));
    drift = prov.clone();
    drift["artifacts"]
        .as_object_mut()
        .unwrap()
        .remove("checker-identity.json");
    assert!(!provenance_valid(&drift, &names));
    for n in names {
        let b = std::fs::read(path(n)).unwrap();
        assert_eq!(a[n]["length_bytes"], b.len());
        assert_eq!(a[n]["sha256"], hash(&b));
    }
    assert_eq!(read("case.json")["case_id"], "F-04-stale-replay");
    assert!(check(
        &source,
        &read("operations.json"),
        &read("expected-archive.json"),
        &read("expected-visible.json"),
        &read("expected-plan.json"),
        &read("expected-report.json")
    ));
}
#[test]
fn mutations_rejected() {
    let s: Value = serde_json::from_slice(&std::fs::read(path("source.bin")).unwrap()).unwrap();
    let o = read("operations.json");
    let a = read("expected-archive.json");
    let v = read("expected-visible.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    let mut m = o.clone();
    m[3]["result_revision"] = json!(4);
    assert!(!check(&s, &m, &a, &v, &p, &r));
    let mut m = o.clone();
    m[3]["target"] = json!("r-cues/c1.label");
    assert!(!check(&s, &m, &a, &v, &p, &r));
    let mut m = o.clone();
    m[4]["result"] = json!("Mutated");
    assert!(!check(&s, &m, &a, &v, &p, &r));
    let mut m = o.clone();
    m[5]["new"] = json!("Opening 3");
    assert!(!check(&s, &m, &a, &v, &p, &r));
    let mut m = o.clone();
    m.as_array_mut().unwrap().pop();
    assert!(!check(&s, &m, &a, &v, &p, &r));
    let mut m = a.clone();
    m["operations"] = json!([]);
    assert!(!check(&s, &o, &m, &v, &p, &r));
    let mut m = a.clone();
    m["records"][1]["cues"][0]["label"] = json!("Wrong");
    assert!(!check(&s, &o, &m, &v, &p, &r));
    let mut m = r.clone();
    m.as_array_mut().unwrap().push(json!({"drift":true}));
    assert!(!check(&s, &o, &a, &v, &p, &m));
    let mut m = p.clone();
    m["report"] = json!([{"drift":true}]);
    assert!(!check(&s, &o, &a, &v, &m, &r));
}
