use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/f01-same-path-logical")
}
fn read(n: &str) -> Value {
    serde_json::from_slice(&std::fs::read(dir().join(n)).unwrap()).unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn check(s: &Value, a: &Value, o: &Value, p: &Value, r: &Value) -> Result<(), &'static str> {
    let different = "oxidj-f01-different-v1\n";
    let original = "oxidj-f01-content-v1\n";
    let grid = json!({"bpm":"128","anchor_usec":500000});
    let cue = json!({"id":"c0","index":0,"kind":"HotCue","label":"Intro","start_usec":10000000});
    let records = json!([
     {"record_id":"r1","namespace":"mixxx-library-A","path":"/music/track1.flac","external_id":"100","source_revision":"s1","grid":grid,"cue":cue,"content_witness_utf8":original},
     {"record_id":"r2","namespace":"mixxx-library-B","path":"/music/track1.flac","external_id":"200","source_revision":"s1","grid":grid,"cue":cue,"content_witness_utf8":different}
    ]);
    if s != &json!({"case_id":"F-01-same-path-content-conflict","source_kind":"mixxx-library-logical","source_version":"logical-v1","records":records})
    {
        return Err("source contract/immutability mismatch");
    }
    let ops = json!([{"operation_id":"import-a","operation":"import","record_id":"r1","expected_revision":0,"result":"accepted","revision_after":1},{"operation_id":"import-b","operation":"import-candidate","record_id":"r2","expected_revision":1,"result":"identity-conflict","revision_after":1,"reason":"path-content-disagreement"}]);
    if o != &ops {
        return Err("operations mismatch");
    }
    if a != &json!({"track_id":"t1","revision":1,"source_records":records,"candidate_records":["r1","r2"],"reconciled_records":["r1"],"operations":ops,"current_location_record_id":"r1","effective_grid":grid,"effective_cue":cue,"user_overlays":[]})
    {
        return Err("archive/merge/original mutation mismatch");
    }
    let report = json!([{"code":"IDENTITY_CONFLICT","record_id":"r2","element_id":null,"field":"identity","cause_operation_id":null,"action":"BLOCK_EXPORT","details":{"reason":"path-content-disagreement"}}]);
    if r != &report
        || p != &json!({"outcome":"BLOCK_EXPORT","export_policy":"safe-v1","selected_units":[],"withheld_units":["r2"],"report":report,"no_write_expected":true})
    {
        return Err("plan/report mismatch");
    }
    Ok(())
}
#[test]
fn frozen_artifact_set_provenance_and_contract() {
    let names = [
        "case.json",
        "expected-archive.json",
        "expected-plan.json",
        "expected-report.json",
        "expected-visible.json",
        "operations.json",
        "source.bin",
    ];
    let prov = read("provenance.json");
    let src = std::fs::read(dir().join("source.bin")).unwrap();
    assert_eq!(prov["source_length_bytes"], src.len());
    assert_eq!(prov["source_sha256"], hash(&src));
    assert_eq!(prov["exact_artifact_set"], json!(names));
    assert_eq!(prov["case_id"], "F-01-same-path-content-conflict");
    assert_eq!(prov["checker"], "xtask/tests/f01_same_path_logical.rs");
    assert_eq!(prov["checker_version"], "1");
    assert_eq!(prov["classification"], "logical-only-not-A2-evidence");
    assert_eq!(
        prov["source_citation"],
        "docs/adr/005-relationship-fixtures.md §3 F-01"
    );
    assert_eq!(
        prov["source_version"],
        "Mixxx 2.5.6 logical recipe; synthetic evidence, not Mixxx transport bytes or audio"
    );
    let claims_match = |value: &Value| {
        value["case_id"] == "F-01-same-path-content-conflict"
            && value["checker"] == "xtask/tests/f01_same_path_logical.rs"
            && value["checker_version"] == "1"
            && value["classification"] == "logical-only-not-A2-evidence"
            && value["source_citation"] == "docs/adr/005-relationship-fixtures.md §3 F-01"
            && value["source_version"]
                == "Mixxx 2.5.6 logical recipe; synthetic evidence, not Mixxx transport bytes or audio"
    };
    assert!(claims_match(&prov));
    assert_eq!(
        prov.as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        [
            "artifacts",
            "case_id",
            "checker",
            "checker_version",
            "classification",
            "exact_artifact_set",
            "source_citation",
            "source_length_bytes",
            "source_sha256",
            "source_version"
        ]
        .into_iter()
        .collect()
    );
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
    let s: Value = serde_json::from_slice(&src).unwrap();
    let a = read("expected-archive.json");
    let o = read("operations.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    assert_eq!(
        read("case.json"),
        json!({"case_id":"F-01-same-path-content-conflict","classification":"logical-only-not-A2-evidence","contract_version":"logical-v1","export_policy":"safe-v1","ordered_element_ids":["c0"],"ordered_record_ids":["r1","r2"],"source_kind":"mixxx-library-logical","source_version":"logical-v1","target_format":"mixxx-library-logical"})
    );
    assert_eq!(read("expected-visible.json"), Value::Null);
    assert_eq!(prov["checker"], "xtask/tests/f01_same_path_logical.rs");
    assert_eq!(prov["checker_version"], "1");
    check(&s, &a, &o, &p, &r).unwrap();
    let mut changed_provenance = prov.clone();
    changed_provenance["source_citation"] = json!("docs/adr/005-relationship-fixtures.md §4");
    assert!(!claims_match(&changed_provenance));
}
#[test]
fn literal_mutation_controls_reject_merge_source_mutation_missing_candidate_and_report_changes() {
    let s = read("source.bin");
    let a = read("expected-archive.json");
    let o = read("operations.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    let mut x = a.clone();
    x["reconciled_records"] = json!(["r1", "r2"]);
    assert!(check(&s, &x, &o, &p, &r).is_err());
    let mut x = s.clone();
    x["records"][0]["content_witness_utf8"] = json!("mutated\n");
    assert!(check(&x, &a, &o, &p, &r).is_err());
    let mut x = s.clone();
    x["records"][1]["path"] = json!("/music/drift.flac");
    assert!(check(&x, &a, &o, &p, &r).is_err());
    let mut x = s.clone();
    x["records"][1]["content_witness_utf8"] = json!("drift\n");
    assert!(check(&x, &a, &o, &p, &r).is_err());
    let mut x = a.clone();
    x["candidate_records"] = json!(["r1"]);
    assert!(check(&s, &x, &o, &p, &r).is_err());
    let mut x = r.clone();
    x[0]["record_id"] = json!("r1");
    assert!(check(&s, &a, &o, &p, &x).is_err());
    let mut x = r.clone();
    x[0]["details"]["reason"] = json!("unconfirmed-identity");
    assert!(check(&s, &a, &o, &p, &x).is_err());
    let mut x = p.clone();
    x["no_write_expected"] = json!(false);
    assert!(check(&s, &a, &o, &x, &r).is_err());
}
