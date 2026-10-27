use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2 as _;
use std::path::PathBuf;

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/f04-analysis-logical")
        .join(name)
}
fn read(name: &str) -> Value {
    let local = path(name);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let selected = if local.is_file() { local } else { root };
    serde_json::from_slice(&std::fs::read(selected).unwrap()).unwrap()
}

// Independent logical oracle: stale analysis is rejected before any state mutation.
fn stale_rejected(
    source: &Value,
    candidate: &Value,
    before: &Value,
    after: &Value,
    result: &Value,
) -> bool {
    source
        == &json!({"source_revision":"s1","content_revision":3,"grid":{"bpm":"128","first_beat_frame":96000,"sample_rate_hz":48000,"source":"ANALYZER"},"cues":[{"color":[255,0,0],"flags":0,"id":"c0","index":0,"kind":"HotCue","label":"Intro","start_usec":10000000}]})
        && candidate
            == &json!({"operation_id":"a-stale","target":"r-grid/grid.bpm","expected_revision":3,"analyzer":"m1-synthetic","analyzer_version":"1","sequence":1,"source_revision":"s0","content_revision":2,"bpm":"129"})
        && before == after
        && before["revision"] == 3
        && result
            == &json!({"operation_id":"a-stale","result":"Conflict","reason":"stale-source-or-content-revision","state_changed":false})
}

#[test]
fn stale_source_analysis_rejects_without_mutation() {
    let source = json!({"source_revision":"s1","content_revision":3,"grid":{"bpm":"128","first_beat_frame":96000,"sample_rate_hz":48000,"source":"ANALYZER"},"cues":[{"color":[255,0,0],"flags":0,"id":"c0","index":0,"kind":"HotCue","label":"Intro","start_usec":10000000}]});
    let candidate = json!({"operation_id":"a-stale","target":"r-grid/grid.bpm","expected_revision":3,"analyzer":"m1-synthetic","analyzer_version":"1","sequence":1,"source_revision":"s0","content_revision":2,"bpm":"129"});
    let before = read("f04-analysis-stale-before.json");
    let after = read("f04-analysis-stale-after.json");
    let result = read("f04-analysis-stale-result.json");
    assert!(stale_rejected(
        &source, &candidate, &before, &after, &result
    ));
}

#[test]
fn mutation_controls_reject_revision_or_state_drift() {
    let source = json!({"source_revision":"s1","content_revision":3,"grid":{"bpm":"128","first_beat_frame":96000,"sample_rate_hz":48000,"source":"ANALYZER"},"cues":[{"color":[255,0,0],"flags":0,"id":"c0","index":0,"kind":"HotCue","label":"Intro","start_usec":10000000}]});
    let candidate = json!({"operation_id":"a-stale","target":"r-grid/grid.bpm","expected_revision":3,"analyzer":"m1-synthetic","analyzer_version":"1","sequence":1,"source_revision":"s0","content_revision":2,"bpm":"129"});
    let before = read("f04-analysis-stale-before.json");
    let after = read("f04-analysis-stale-after.json");
    let result = read("f04-analysis-stale-result.json");
    let mut m = source.clone();
    m["source_revision"] = json!("s0");
    assert!(!stale_rejected(&m, &candidate, &before, &after, &result));
    let mut m = source.clone();
    m["content_revision"] = json!(2);
    assert!(!stale_rejected(&m, &candidate, &before, &after, &result));
    let mut m = after.clone();
    m["revision"] = json!(4);
    assert!(!stale_rejected(&source, &candidate, &before, &m, &result));
    let mut m = after.clone();
    m["grid"]["bpm"] = json!("129");
    assert!(!stale_rejected(&source, &candidate, &before, &m, &result));
    let mut m = after.clone();
    m["analysis"]
        .as_array_mut()
        .unwrap()
        .push(candidate.clone());
    assert!(!stale_rejected(&source, &candidate, &before, &m, &result));
    let mut m = result.clone();
    m["state_changed"] = json!(true);
    assert!(!stale_rejected(&source, &candidate, &before, &after, &m));
}
