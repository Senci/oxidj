use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f06b-logical")
        .join(name)
}

fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(fixture(name)).unwrap()).unwrap()
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn check_provenance(
    provenance: &Value,
    identity: &Value,
    live_checker: &[u8],
) -> Result<(), &'static str> {
    let expected_identity = json!({
        "path": "xtask/tests/f06b_logical.rs",
        "version": "f06b-logical-checker-v1",
        "sha256": digest(live_checker)
    });
    if identity != &expected_identity
        || provenance["checker_identity"] != *identity
        || provenance["case_id"] != "F-06b"
        || provenance["classification"]
            != "logical-only-OxiDJ-declared-relationship-not-A2-evidence"
        || provenance["source_citation"] != "docs/adr/005-relationship-fixtures.md §3 F-06b"
    {
        return Err("checker provenance mismatch");
    }
    let expected = [
        "case.json",
        "expected-archive.json",
        "expected-plan.json",
        "expected-report.json",
        "expected-visible.json",
        "operations.json",
        "source.bin",
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
        != expected
    {
        return Err("provenance artifact set mismatch");
    }
    let actual = std::fs::read_dir(fixture("."))
        .map_err(|_| "fixture missing")?
        .map(|entry| entry.map_err(|_| "entry unreadable"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>();
    let expected_files = expected
        .iter()
        .map(|name| name.to_string())
        .chain(["provenance.json".to_owned()])
        .collect::<BTreeSet<_>>();
    if actual != expected_files {
        return Err("live fixture artifact set mismatch");
    }
    for (name, info) in artifacts {
        let bytes = std::fs::read(fixture(name)).map_err(|_| "artifact missing")?;
        if info["path"] != name.as_str()
            || info["length_bytes"] != bytes.len()
            || info["sha256"] != digest(&bytes)
        {
            return Err("artifact digest mismatch");
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
    let beats = json!([
        {"id":"b0","frame":0,"enabled":true,"source":"ANALYZER"},
        {"id":"b1","frame":120000,"enabled":true,"source":"ANALYZER"},
        {"id":"b2","frame":240000,"enabled":true,"source":"ANALYZER"}
    ]);
    let cue = json!({"id":"c0","kind":"HotCue","index":0,"start_usec":3000000,"label":"Transition","color":[0,255,0],"flags":0});
    let relation = json!({"cue_id":"c0","anchor_element_id":"b1","offset_usec":500000});
    let operation = json!({"operation_id":"e1","record_id":"r-grid","target_id":"b1","expected_revision":0,"old_value":{"id":"b1","frame":120000,"enabled":true,"source":"ANALYZER"},"new_value":null,"result":"accepted"});
    let entry = json!({"code":"MISSING_ANCHOR","target_format":"mixxx-cueinfo-logical","record_id":"r-cues","element_id":"c0","field":"anchor","cause_operation_id":"e1","action":"WITHHOLD","details":{"anchor_element_id":"b1"}});

    if source["beats"] != beats
        || source["cue"] != cue
        || source["relationship"] != relation
        || source["sample_rate_hz"] != 48000
        || source["snapshot_kind"] != "OXIDJ_ONLY_DECLARED"
    {
        return Err("source mismatch");
    }
    if archive["source_beats"] != beats
        || archive["source_cue"] != cue
        || archive["source_relationship"] != relation
        || archive["operations"] != json!([operation])
        || archive["revision"] != 1
        || archive["visible_beats"] != json!([beats[0].clone(), beats[2].clone()])
        || archive["effective_cue"] != cue
        || archive["effective_relationship"] != relation
        || archive["unresolved_relationships"] != json!([relation])
    {
        return Err("archive mismatch");
    }
    if visible["beats"] != json!([beats[0].clone(), beats[2].clone()])
        || visible["cues"] != json!([])
        || visible["report"] != json!([entry.clone()])
        || report != &json!([entry.clone()])
    {
        return Err("visible/report mismatch");
    }
    if plan["case_id"] != "F-06b"
        || plan["outcome"] != "ALLOW_EXPORT"
        || plan["selected_units"] != json!(["b0", "b2"])
        || plan["withheld_units"] != json!(["c0"])
        || plan["report"] != json!([entry])
        || plan["no_write_expected"] != false
    {
        return Err("plan mismatch");
    }
    if reimport["beats"] != json!([beats[0].clone(), beats[2].clone()])
        || reimport["cues"] != json!([])
        || reimport["relationships"] != json!([])
        || reimport["report"] != json!([])
    {
        return Err("vendor-only re-import mismatch");
    }
    Ok(())
}

#[test]
fn f06b_frozen_logical_fixture_matches_independent_contract() {
    let identity = read("checker-identity.json");
    let live_checker =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f06b_logical.rs"))
            .unwrap();
    check_provenance(&read("provenance.json"), &identity, &live_checker).unwrap();
    let source_bytes = std::fs::read(fixture("source.bin")).unwrap();
    let source: Value = serde_json::from_slice(&source_bytes).unwrap();
    let mut canonical = serde_json::to_vec(&source).unwrap();
    canonical.push(b'\n');
    assert_eq!(
        source_bytes, canonical,
        "source.bin must be canonical JSON bytes"
    );

    let archive = read("expected-archive.json");
    let visible = read("expected-visible.json");
    let plan = read("expected-plan.json");
    let report = read("expected-report.json");
    let reimport = read("vendor-reimport.json");
    assert_eq!(read("case.json")["case_id"], "F-06b");
    assert_eq!(read("operations.json"), archive["operations"]);
    check(&source, &archive, &visible, &plan, &report, &reimport).unwrap();
}

#[test]
fn f06b_mutations_reject_loss_report_changes_and_resurrection() {
    let source: Value =
        serde_json::from_slice(&std::fs::read(fixture("source.bin")).unwrap()).unwrap();
    let archive = read("expected-archive.json");
    let visible = read("expected-visible.json");
    let plan = read("expected-plan.json");
    let report = read("expected-report.json");
    let reimport = read("vendor-reimport.json");

    let mut missing_report = visible.clone();
    missing_report["report"] = json!([]);
    assert!(
        check(
            &source,
            &archive,
            &missing_report,
            &plan,
            &report,
            &reimport
        )
        .is_err()
    );

    let mut altered_cause = report.clone();
    altered_cause[0]["cause_operation_id"] = json!("other");
    assert!(
        check(
            &source,
            &archive,
            &visible,
            &plan,
            &altered_cause,
            &reimport
        )
        .is_err()
    );

    let mut resurrected = reimport.clone();
    resurrected["cues"] = json!([{"id":"c0"}]);
    assert!(check(&source, &archive, &visible, &plan, &report, &resurrected).is_err());

    let mut resurrected_relationship = reimport.clone();
    resurrected_relationship["relationships"] = json!([{"cue_id":"c0","anchor_element_id":"b1"}]);
    assert!(
        check(
            &source,
            &archive,
            &visible,
            &plan,
            &report,
            &resurrected_relationship
        )
        .is_err()
    );

    let identity = read("checker-identity.json");
    let live_checker =
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/f06b_logical.rs"))
            .unwrap();
    let provenance = read("provenance.json");
    for (key, value) in [
        ("path", json!("xtask/tests/other.rs")),
        ("version", json!("drift")),
        ("sha256", json!("wrong")),
    ] {
        let mut drift = identity.clone();
        drift[key] = value;
        assert!(check_provenance(&provenance, &drift, &live_checker).is_err());
    }
    assert!(check_provenance(&provenance, &identity, b"mutated checker").is_err());
    let mut bad = provenance.clone();
    bad["artifacts"]["unexpected.json"] = json!({});
    assert!(check_provenance(&bad, &identity, &live_checker).is_err());
}
