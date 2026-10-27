use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f04-cue-order-logical")
        .join(name)
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(path(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn expected_source() -> Value {
    json!({"grid":{"bpm":"128","first_beat_frame":96000,"sample_rate_hz":48000,"source":"ANALYZER"},"cues":[
        {"color":[255,0,0],"flags":0,"id":"c0","index":0,"kind":"HotCue","label":"Intro","start_usec":10000000},
        {"color":[0,0,255],"flags":0,"id":"c1","index":1,"kind":"HotCue","label":"Drop","start_usec":20000000},
        {"end_usec":25000000,"flags":0,"id":"c2","kind":"Loop","label":"Loop 1","start_usec":15000000},
        {"flags":0,"id":"c3","kind":"MainCue","start_usec":5000000}
    ]})
}
fn provenance_valid(prov: &Value) -> bool {
    let names = [
        "case.json",
        "operations.json",
        "expected-archive.json",
        "expected-visible.json",
        "expected-plan.json",
        "expected-report.json",
        "source.bin",
    ];
    if prov["case_id"] != "F-04-cue-order-edit"
        || prov["classification"] != "logical-only-not-A2-evidence"
        || prov["checker"] != "independent-literal-v1"
    {
        return false;
    }
    let Ok(source) = std::fs::read(path("source.bin")) else {
        return false;
    };
    if prov["source_sha256"] != hash(&source) || prov["source_length_bytes"] != source.len() {
        return false;
    }
    let Some(artifacts) = prov["artifacts"].as_object() else {
        return false;
    };
    if artifacts.len() != names.len() {
        return false;
    }
    names.iter().all(|n| {
        let Ok(bytes) = std::fs::read(path(n)) else {
            return false;
        };
        artifacts[*n]["path"] == *n
            && artifacts[*n]["length_bytes"] == bytes.len()
            && artifacts[*n]["sha256"] == hash(&bytes)
    })
}
fn expected_ops() -> Value {
    json!([
        {"operation_id":"e1","target":"r-cues/order","expected_revision":0,"old":["c0","c1","c2","c3"],"new":["c1","c0","c2","c3"],"result_revision":1},
        {"operation_id":"e2","target":"r-cues/c0.label","expected_revision":1,"old":"Intro","new":"Opening","result_revision":2}
    ])
}
// Independent literal oracle. It deliberately calls no importer, editor, or exporter.
fn check(
    source_bytes: &[u8],
    ops: &Value,
    archive: &Value,
    visible: &Value,
    plan: &Value,
    report: &Value,
) -> bool {
    let source: Value = match serde_json::from_slice(source_bytes) {
        Ok(v) => v,
        Err(_) => return false,
    };
    if source != expected_source()
        || source_bytes != format!("{}\n", source).as_bytes()
        || ops != &expected_ops()
    {
        return false;
    }
    let initial = &source["cues"];
    let effective = json!([
        {"color":[0,0,255],"flags":0,"id":"c1","index":1,"kind":"HotCue","label":"Drop","start_usec":20000000},
        {"color":[255,0,0],"flags":0,"id":"c0","index":0,"kind":"HotCue","label":"Opening","start_usec":10000000},
        {"end_usec":25000000,"flags":0,"id":"c2","kind":"Loop","label":"Loop 1","start_usec":15000000},
        {"flags":0,"id":"c3","kind":"MainCue","start_usec":5000000}
    ]);
    if archive["revision"] != 2
        || archive["operations"] != expected_ops()
        || archive["source_cues"] != *initial
        || archive["source_grid"] != source["grid"]
        || archive["report"] != json!([])
    {
        return false;
    }
    let records = match archive["records"].as_array() {
        Some(x) if x.len() == 2 => x,
        _ => return false,
    };
    let expected_grid_record = json!({"analysis":[],"effective_revision":0,"grid":source["grid"],"origin":"SYNTHETIC","overlays":[],"record_id":"r-grid","source_namespace":"F-03","source_provenance":{"case_id":"F-03","source_kind":"mixxx-cueinfo-logical","source_version":"logical-v1"},"source_revision":"s1","track_id":"t1"});
    let expected_cue_record = json!({"analysis":[],"cues":effective,"effective_revision":2,"origin":"SYNTHETIC","overlays":[{"operation_id":"e1","field":"order","value":["c1","c0","c2","c3"],"origin":"USER"},{"operation_id":"e2","field":"c0.label","value":"Opening","origin":"USER"}],"original_cues":initial,"record_id":"r-cues","source_namespace":"F-03","source_provenance":{"case_id":"F-03","source_kind":"mixxx-cueinfo-logical","source_version":"logical-v1"},"source_revision":"s1","track_id":"t1"});
    records[0] == expected_grid_record
        && records[1] == expected_cue_record
        && visible == &json!({"grid":source["grid"],"cues":effective,"report":[]})
        && plan
            == &json!({"case_id":"F-04-cue-order-edit","contract_version":"logical-v1","decision_phase":"pre-write","export_policy":"safe-v1","no_write_expected":false,"outcome":"ALLOW_EXPORT","report":[],"selected_units":["grid","c1","c0","c2","c3"],"target_format":"mixxx-cueinfo-logical","target_write":"permitted-but-not-performed-by-fixture-harness","withheld_units":[]})
        && report == &json!([])
}

#[test]
fn frozen_artifacts_independent_checker_and_mutation_controls() {
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
    assert!(provenance_valid(&prov));
    let mut changed_provenance = prov.clone();
    changed_provenance["source_sha256"] = json!("0");
    assert!(!provenance_valid(&changed_provenance));
    let artifacts = prov["artifacts"].as_object().unwrap();
    assert_eq!(artifacts.len(), names.len());
    let mut actual: Vec<_> = std::fs::read_dir(path("."))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    actual.sort();
    let mut expected: Vec<_> = names
        .iter()
        .map(|n| n.to_string())
        .chain(["provenance.json".to_owned()])
        .collect();
    expected.sort();
    assert_eq!(actual, expected);
    for n in names {
        let b = std::fs::read(path(n)).unwrap();
        assert_eq!(artifacts[n]["length_bytes"], b.len());
        assert_eq!(artifacts[n]["sha256"], hash(&b));
    }
    let ops = read("operations.json");
    let archive = read("expected-archive.json");
    let visible = read("expected-visible.json");
    let plan = read("expected-plan.json");
    let report = read("expected-report.json");
    assert!(check(&source, &ops, &archive, &visible, &plan, &report));
    let mut wrong_order = visible.clone();
    wrong_order["cues"][0] = visible["cues"][1].clone();
    wrong_order["cues"][1] = visible["cues"][0].clone();
    assert!(!check(
        &source,
        &ops,
        &archive,
        &wrong_order,
        &plan,
        &report
    ));
    let mut wrong_target = ops.clone();
    wrong_target[1]["target"] = json!("r-cues/c1.label");
    assert!(!check(
        &source,
        &wrong_target,
        &archive,
        &visible,
        &plan,
        &report
    ));
    let mut drift = visible.clone();
    drift["cues"][2]["end_usec"] = json!(25000001);
    assert!(!check(&source, &ops, &archive, &drift, &plan, &report));
    let mut revision = archive.clone();
    revision["revision"] = json!(3);
    assert!(!check(&source, &ops, &revision, &visible, &plan, &report));
    let mut changed_source: Value = serde_json::from_slice(&source).unwrap();
    changed_source["cues"][0]["label"] = json!("Changed");
    assert!(!check(
        &serde_json::to_vec(&changed_source).unwrap(),
        &ops,
        &archive,
        &visible,
        &plan,
        &report
    ));
    let mut changed_archive = archive.clone();
    changed_archive["records"][1]["cues"][0]["label"] = json!("Changed");
    assert!(!check(
        &source,
        &ops,
        &changed_archive,
        &visible,
        &plan,
        &report
    ));
    assert!(!check(
        &source,
        &ops,
        &archive,
        &visible,
        &plan,
        &json!([{"unexpected":true}])
    ));
}
