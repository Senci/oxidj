use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f12-logical")
        .join(name)
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(fixture(name)).unwrap()).unwrap()
}
fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn expected_cues() -> Vec<Value> {
    (0..40).map(|i| json!({"id":format!("c{i}"),"kind":"HotCue","index":i,"start_usec":(i+1)*1_000_000,"label":format!("H{i:02}"),"flags":0})).collect()
}
fn expected_checker_identity() -> Value {
    json!({"path":"xtask/tests/f12_logical.rs","version":"f12-logical-checker-v1"})
}
fn check_provenance(provenance: &Value, source: &[u8]) -> Result<(), &'static str> {
    let identity: Value = serde_json::from_slice(
        &std::fs::read(fixture("checker-identity.json")).map_err(|_| "checker identity missing")?,
    )
    .map_err(|_| "invalid checker identity")?;
    let live =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f12_logical.rs"))
            .map_err(|_| "checker source missing")?;
    let live_digest = digest(&live);
    let mut expected = expected_checker_identity();
    expected["sha256"] = json!(live_digest);
    if identity != expected || provenance["checker_identity"] != identity {
        return Err("checker identity mismatch");
    }
    if provenance["source_sha256"] != digest(source)
        || provenance["source_length_bytes"] != source.len()
    {
        return Err("provenance source mismatch");
    }
    let expected_files = [
        "case.json",
        "source.bin",
        "expected-archive.json",
        "expected-visible.json",
        "expected-plan.json",
        "expected-report.json",
        "vendor-reimport.json",
        "checker-identity.json",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let artifacts = provenance["artifacts"]
        .as_object()
        .ok_or("provenance artifacts missing")?;
    if artifacts
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected_files
    {
        return Err("provenance artifact set mismatch");
    }
    let actual = std::fs::read_dir(fixture("."))
        .map_err(|_| "fixture missing")?
        .map(|e| e.map_err(|_| "entry unreadable"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>();
    let expected_live = expected_files
        .iter()
        .map(|s| s.to_string())
        .chain(["provenance.json".to_owned()])
        .collect::<BTreeSet<_>>();
    if actual != expected_live {
        return Err("live fixture artifact set mismatch");
    }
    for (name, info) in artifacts {
        let bytes = std::fs::read(fixture(name)).map_err(|_| "artifact missing")?;
        if info["path"] != name.as_str()
            || info["length_bytes"] != bytes.len()
            || info["sha256"] != digest(&bytes)
        {
            return Err("artifact provenance mismatch");
        }
    }
    Ok(())
}
fn check(
    source: &Value,
    archive: &Value,
    visible: &Value,
    plan: &Value,
    report: &Value,
    reimport: &Value,
) -> Result<(), &'static str> {
    let cues = expected_cues();
    let entries:Vec<Value>=(38..40).map(|i|json!({"code":"HOTCUE_CAPACITY","target_format":"mixxx-cueinfo-logical","record_id":"r-cues","element_id":format!("c{i}"),"field":"hotcue_index","cause_operation_id":null,"action":"WITHHOLD","details":{"index":i,"maximum_index":37}})).collect();
    if source["case_id"] != "F-12"
        || source["source_kind"] != "mixxx-cueinfo-logical"
        || source["source_version"] != "logical-v1"
        || source["cues"] != json!(cues)
    {
        return Err("source mismatch");
    }
    if archive["source_cues"] != json!(cues)
        || archive["track_id"] != "t1"
        || archive["revision"] != 0
        || archive["operations"] != json!([])
        || archive["records"][0]["cues"] != json!(cues)
        || archive["records"][0]["record_id"] != "r-cues"
    {
        return Err("archive mismatch");
    }
    if visible["cues"] != json!(cues[..38])
        || visible["report"] != json!([])
        || report != &json!(entries)
    {
        return Err("visible/report mismatch");
    }
    if plan["case_id"] != "F-12"
        || plan["outcome"] != "ALLOW_EXPORT"
        || plan["export_policy"] != "safe-v1"
        || plan["selected_units"] != json!((0..38).map(|i| format!("c{i}")).collect::<Vec<_>>())
        || plan["withheld_units"] != json!(["c38", "c39"])
        || plan["report"] != json!(entries)
        || plan["no_write_expected"] != false
    {
        return Err("plan mismatch");
    }
    if reimport["cues"] != json!(cues[..38]) || reimport["report"] != json!([]) {
        return Err("vendor-only re-import mismatch");
    }
    Ok(())
}
#[test]
fn f12_artifacts_provenance_and_independent_contract_are_frozen() {
    let src = std::fs::read(fixture("source.bin")).unwrap();
    let provenance = read("provenance.json");
    check_provenance(&provenance, &src).unwrap();
    assert_eq!(provenance["source_sha256"], digest(&src));
    assert_eq!(provenance["source_length_bytes"], src.len());
    for (name, info) in provenance["artifacts"].as_object().unwrap() {
        let b = std::fs::read(fixture(name)).unwrap();
        assert_eq!(info["length_bytes"], b.len());
        assert_eq!(info["sha256"], digest(&b));
    }
    let source: Value = serde_json::from_slice(&src).unwrap();
    let a = read("expected-archive.json");
    let v = read("expected-visible.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    let re = read("vendor-reimport.json");
    check(&source, &a, &v, &p, &r, &re).unwrap();
}
#[test]
fn f12_mutations_reject_archive_renumber_resurrection_and_bad_loss_entries() {
    let s = read("source.bin");
    let a = read("expected-archive.json");
    let v = read("expected-visible.json");
    let p = read("expected-plan.json");
    let r = read("expected-report.json");
    let re = read("vendor-reimport.json");
    let mut x = a.clone();
    x["records"][0]["cues"].as_array_mut().unwrap().pop();
    assert!(check(&s, &x, &v, &p, &r, &re).is_err());
    let mut x = v.clone();
    x["cues"][37]["index"] = json!(38);
    assert!(check(&s, &a, &x, &p, &r, &re).is_err());
    let mut x = v.clone();
    x["cues"].as_array_mut().unwrap().push(json!({"id":"c38"}));
    assert!(check(&s, &a, &x, &p, &r, &re).is_err());
    let mut x = r.clone();
    x.as_array_mut().unwrap().pop();
    assert!(check(&s, &a, &v, &p, &x, &re).is_err());
    let mut x = r.clone();
    x[0]["details"]["maximum_index"] = json!(38);
    assert!(check(&s, &a, &v, &p, &x, &re).is_err());
    let mut x = re.clone();
    x["cues"].as_array_mut().unwrap().push(json!({"id":"c38"}));
    assert!(check(&s, &a, &v, &p, &r, &x).is_err());
}
#[test]
fn f12_provenance_rejects_checker_identity_and_artifact_drift() {
    let source = std::fs::read(fixture("source.bin")).unwrap();
    let provenance = read("provenance.json");
    for key in ["path", "version", "sha256"] {
        let mut changed = provenance.clone();
        changed["checker_identity"][key] = json!("drift");
        assert!(check_provenance(&changed, &source).is_err());
    }
    let mut extra = provenance.clone();
    extra["artifacts"]["unexpected.json"] = json!({});
    assert!(check_provenance(&extra, &source).is_err());
    let mut bad_digest = provenance;
    bad_digest["artifacts"]["source.bin"]["sha256"] = json!("00");
    assert!(check_provenance(&bad_digest, &source).is_err());
}
