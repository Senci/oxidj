use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::{Value, json};
use sha2 as _;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
fn base(name: &str) -> PathBuf {
    fixture("f04-analysis-logical").join(name)
}
fn read(path: PathBuf) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[allow(clippy::too_many_arguments)]
fn check(
    source: &Value,
    candidate: &Value,
    archive: &Value,
    intermediate: &Value,
    visible: &Value,
    plan: &Value,
    report: &Value,
    provenance: &Value,
    files: &[String],
    before: &Value,
    after: &Value,
    result: &Value,
) -> bool {
    let expected_source = read(fixture("f04-analysis-stale-source.json"));
    let expected_candidate = read(fixture("f04-analysis-stale-candidate.json"));
    let baseline_archive = read(base("expected-archive.json"));
    let baseline_intermediate = read(base("expected-intermediate.json"));
    let baseline_visible = read(base("expected-visible.json"));
    let baseline_plan = read(base("expected-plan.json"));
    let baseline_report = read(base("expected-report.json"));
    let baseline_provenance = read(base("provenance.json"));
    let operations = read(base("operations.json"));
    let expected_before = json!({"archive":baseline_archive,"intermediate":baseline_intermediate,"visible":baseline_visible,"revision":5,"operations":operations});
    let mut expected_files: Vec<String> = [
        "case.json",
        "operations.json",
        "expected-archive.json",
        "expected-intermediate.json",
        "expected-visible.json",
        "expected-plan.json",
        "expected-report.json",
        "source.bin",
        "provenance.json",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    expected_files.extend(
        [
            "f04-analysis-stale-source.json",
            "f04-analysis-stale-candidate.json",
            "f04-analysis-stale-before.json",
            "f04-analysis-stale-after.json",
            "f04-analysis-stale-result.json",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    expected_files.sort();
    let mut actual = files.to_vec();
    actual.sort();
    source == &expected_source
        && candidate == &expected_candidate
        && archive == &expected_before["archive"]
        && intermediate == &expected_before["intermediate"]
        && visible == &expected_before["visible"]
        && plan == &baseline_plan
        && report == &baseline_report
        && provenance == &baseline_provenance
        && actual == expected_files
        && before == &expected_before
        && after == before
        && result
            == &json!({"operation_id":"a-stale","result":"Conflict","reason":"stale-source-or-content-revision","state_changed":false})
}
#[allow(clippy::type_complexity)]
fn inputs() -> (
    Value,
    Value,
    Value,
    Value,
    Value,
    Value,
    Value,
    Value,
    Vec<String>,
    Value,
    Value,
    Value,
) {
    let archive = read(base("expected-archive.json"));
    let intermediate = read(base("expected-intermediate.json"));
    let visible = read(base("expected-visible.json"));
    let plan = read(base("expected-plan.json"));
    let report = read(base("expected-report.json"));
    let provenance = read(base("provenance.json"));
    let operations = read(base("operations.json"));
    let source = read(fixture("f04-analysis-stale-source.json"));
    let candidate = read(fixture("f04-analysis-stale-candidate.json"));
    let before = json!({"archive":archive,"intermediate":intermediate,"visible":visible,"revision":5,"operations":operations});
    let after = read(fixture("f04-analysis-stale-after.json"));
    let result = read(fixture("f04-analysis-stale-result.json"));
    let files = [
        "case.json",
        "operations.json",
        "expected-archive.json",
        "expected-intermediate.json",
        "expected-visible.json",
        "expected-plan.json",
        "expected-report.json",
        "source.bin",
        "provenance.json",
        "f04-analysis-stale-source.json",
        "f04-analysis-stale-candidate.json",
        "f04-analysis-stale-before.json",
        "f04-analysis-stale-after.json",
        "f04-analysis-stale-result.json",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    (
        source,
        candidate,
        archive,
        intermediate,
        visible,
        plan,
        report,
        provenance,
        files,
        before,
        after,
        result,
    )
}
#[test]
fn stale_source_analysis_rejects_without_mutation() {
    let (s, c, a, i, v, p, r, pr, f, b, af, rs) = inputs();
    assert!(check(&s, &c, &a, &i, &v, &p, &r, &pr, &f, &b, &af, &rs));
}
#[test]
fn mutation_controls_reject_all_drift() {
    let (s, c, a, i, v, p, r, pr, f, b, af, rs) = inputs();
    macro_rules! rejected {
        ($x:expr) => {
            assert!(!check(
                &$x.0, &$x.1, &$x.2, &$x.3, &$x.4, &$x.5, &$x.6, &$x.7, &$x.8, &$x.9, &$x.10,
                &$x.11
            ));
        };
    }
    let mut x = (
        s.clone(),
        c.clone(),
        a.clone(),
        i.clone(),
        v.clone(),
        p.clone(),
        r.clone(),
        pr.clone(),
        f.clone(),
        b.clone(),
        af.clone(),
        rs.clone(),
    );
    x.0["source_revision"] = json!("s0");
    rejected!(x);
    let mut x = (
        s.clone(),
        c.clone(),
        a.clone(),
        i.clone(),
        v.clone(),
        p.clone(),
        r.clone(),
        pr.clone(),
        f.clone(),
        b.clone(),
        af.clone(),
        rs.clone(),
    );
    x.0["content_revision"] = json!(2);
    rejected!(x);
    let mut x = (
        s.clone(),
        c.clone(),
        a.clone(),
        i.clone(),
        v.clone(),
        p.clone(),
        r.clone(),
        pr.clone(),
        f.clone(),
        b.clone(),
        af.clone(),
        rs.clone(),
    );
    x.1["source_revision"] = json!("s1");
    rejected!(x);
    let mut x = (
        s.clone(),
        c.clone(),
        a.clone(),
        i.clone(),
        v.clone(),
        p.clone(),
        r.clone(),
        pr.clone(),
        f.clone(),
        b.clone(),
        af.clone(),
        rs.clone(),
    );
    x.1["content_revision"] = json!(3);
    rejected!(x);
    let mut x = (
        s.clone(),
        c.clone(),
        a.clone(),
        i.clone(),
        v.clone(),
        p.clone(),
        r.clone(),
        pr.clone(),
        f.clone(),
        b.clone(),
        af.clone(),
        rs.clone(),
    );
    x.9["revision"] = json!(4);
    rejected!(x);
    let mut x = (
        s.clone(),
        c.clone(),
        a.clone(),
        i.clone(),
        v.clone(),
        p.clone(),
        r.clone(),
        pr.clone(),
        f.clone(),
        b.clone(),
        af.clone(),
        rs.clone(),
    );
    x.10["archive"]["records"][0]["user_history"] = json!([]);
    rejected!(x);
    let mut x = (
        s.clone(),
        c.clone(),
        a.clone(),
        i.clone(),
        v.clone(),
        p.clone(),
        r.clone(),
        pr.clone(),
        f.clone(),
        b.clone(),
        af.clone(),
        rs.clone(),
    );
    x.10["visible"]["grid"]["bpm"] = json!("130");
    rejected!(x);
    let mut x = (
        s.clone(),
        c.clone(),
        a.clone(),
        i.clone(),
        v.clone(),
        p.clone(),
        r.clone(),
        pr.clone(),
        f.clone(),
        b.clone(),
        af.clone(),
        rs.clone(),
    );
    x.7["checker"] = json!("drift");
    rejected!(x);
    let mut x = (
        s.clone(),
        c.clone(),
        a.clone(),
        i.clone(),
        v.clone(),
        p.clone(),
        r.clone(),
        pr.clone(),
        f.clone(),
        b.clone(),
        af.clone(),
        rs.clone(),
    );
    x.8.push("unexpected.json".into());
    rejected!(x);
    let mut x = (s, c, a, i, v, p, r, pr, f, b, af, rs);
    x.11["state_changed"] = json!(true);
    rejected!(x);
}
