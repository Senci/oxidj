//! F-09 provenance for bounded synthetic F-02/F-07 operation oracles, not qualification.
use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

type Files = BTreeMap<String, Vec<u8>>;
const MANIFEST: &str = "tests/fixtures/f09-operation-provenance.json";
const MANIFEST_SHA256: &str = "94e0ecd775a066599111e6098e4d93729d880f64963b3d6329ae4500585f8d80";
const TRACK_NAME_HELPER: &str = "xtask/tests/common/f02_track_name_oracle_support.rs";
const XML_TREE_HELPER: &str = "xtask/tests/common/xml_tree.rs";
const TRACK_ATTRIBUTE_ORACLE: &str = "xtask/tests/f02_track_attribute_edit_oracle.rs";

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}
fn is_oracle(path: &str) -> bool {
    (path.starts_with("xtask/tests/f02_") || path.starts_with("xtask/tests/f07_"))
        && path.ends_with("_oracle.rs")
}
fn include(text: &str) -> Option<String> {
    text.split("include_bytes!(\"")
        .nth(1)
        .and_then(|tail| tail.split('\"').next())
        .map(|relative| format!("xtask/tests/{relative}"))
}
fn fixture(source: &str) -> Result<String, String> {
    if let Some(path) = include(source) {
        Ok(path)
    } else if source.contains("f02_playlist_member_append_oracle_shared.rs") {
        Ok("xtask/tests/fixtures/f02-playlist-member-append/source.xml".to_owned())
    } else {
        Err("oracle source does not bind a fixture via include_bytes!".to_owned())
    }
}
fn tests(source: &str) -> Vec<String> {
    source
        .split("#[test]\nfn ")
        .skip(1)
        .map(|s| s.split('(').next().unwrap().to_owned())
        .collect()
}
/// Table-driven checkers: a test whose body is `verify(&ROW)` (an upper-case
/// `const ROW: Case` table row) binds the fixture
/// included by `const ROW: Case`; any other test binds the file-level fixture.
fn test_fixture(source: &str, test: &str) -> Result<String, String> {
    let body = source
        .split(&format!("#[test]\nfn {test}()"))
        .nth(1)
        .ok_or("missing test declaration")?;
    let body = body.split("#[test]\n").next().unwrap();
    let Some(row) = body
        .split("verify(&")
        .nth(1)
        .and_then(|tail| tail.split(')').next())
        .filter(|row| {
            !row.is_empty()
                && row
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        })
    else {
        return fixture(source);
    };
    let row_source = source
        .split(&format!("const {row}: Case = Case {{"))
        .nth(1)
        .ok_or("missing table row")?
        .split("\n};")
        .next()
        .unwrap();
    include(row_source).ok_or_else(|| "table row does not bind a fixture".to_owned())
}
/// Exact (checker, fixture, tests) groups, in checker order then first test use.
fn expected_bindings(files: &Files) -> Result<Vec<(String, String, Vec<String>)>, String> {
    let mut out = vec![];
    for (path, bytes) in files.iter().filter(|(p, _)| is_oracle(p)) {
        let text = std::str::from_utf8(bytes).map_err(|_| "checker source encoding")?;
        let start = out.len();
        for test in tests(text) {
            let source = test_fixture(text, &test)?;
            match out[start..]
                .iter_mut()
                .find(|(_, s, _): &&mut (String, String, Vec<String>)| *s == source)
            {
                Some((_, _, group)) => group.push(test),
                None => out.push((path.clone(), source, vec![test])),
            }
        }
    }
    Ok(out)
}
/// Shared helpers a checker loads with `#[path = "common/..."]`.
fn helpers(source: &str) -> Vec<String> {
    source
        .split("#[path = \"common/")
        .skip(1)
        .map(|tail| format!("xtask/tests/common/{}", tail.split('\"').next().unwrap()))
        .collect()
}
fn snapshot() -> Files {
    let root = root();
    let mut files = Files::new();
    for entry in fs::read_dir(root.join("xtask/tests")).unwrap() {
        let path = entry.unwrap().path();
        let key = path
            .strip_prefix(&root)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        if !is_oracle(&key) {
            continue;
        }
        let bytes = fs::read(&path).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        let names = tests(text);
        assert!(!names.is_empty());
        let mut helper_paths = helpers(text);
        let mut nested = vec![];
        for helper in &helper_paths {
            let helper_text = fs::read_to_string(root.join(helper)).unwrap();
            nested.extend(helpers_nested(&helper_text));
        }
        helper_paths.extend(nested);
        for helper in helper_paths {
            files.insert(
                helper.clone(),
                fs::read(root.join(&helper)).expect("missing shared oracle helper"),
            );
        }
        for test in names {
            let source = test_fixture(text, &test).unwrap();
            for artifact in fs::read_dir(root.join(source).parent().unwrap()).unwrap() {
                let path = artifact.unwrap().path();
                assert!(path.is_file(), "unexpected nested fixture artifact");
                files.insert(
                    path.strip_prefix(&root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
        files.insert(key, bytes);
    }
    files
}
/// Helpers loaded by a helper in `common/` use paths relative to `common/`.
fn helpers_nested(source: &str) -> Vec<String> {
    source
        .split("#[path = \"")
        .skip(1)
        .map(|tail| format!("xtask/tests/common/{}", tail.split('\"').next().unwrap()))
        .collect()
}
fn is_helper(path: &str) -> bool {
    path.starts_with("xtask/tests/common/")
}
fn checker_helpers(text: &str, files: &Files) -> Result<Value, String> {
    let mut paths = helpers(text);
    let mut nested = vec![];
    for path in &paths {
        let bytes = files.get(path).ok_or("missing shared helper source")?;
        let helper_text = std::str::from_utf8(bytes).map_err(|_| "helper source encoding")?;
        nested.extend(helpers_nested(helper_text));
    }
    paths.extend(nested);
    let mut records = vec![];
    for path in paths {
        let bytes = files.get(&path).ok_or("missing shared helper source")?;
        records.push(serde_json::json!({"path": path, "sha256": sha(bytes)}));
    }
    Ok(Value::Array(records))
}
fn manifest() -> Value {
    serde_json::from_slice(
        &fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(MANIFEST)).unwrap(),
    )
    .unwrap()
}
fn validate(manifest: &Value, files: &Files) -> Result<(), String> {
    macro_rules! require {
        ($condition:expr, $message:expr) => {
            if !$condition {
                return Err($message.to_owned());
            }
        };
    }
    require!(
        manifest["schema_version"] == 1,
        "unsupported provenance schema"
    );
    let bindings = manifest["bindings"].as_array().ok_or("missing bindings")?;
    let artifacts = manifest["artifacts"]
        .as_object()
        .ok_or("missing artifacts")?;
    let expected = expected_bindings(files)?;
    let bound: Vec<_> = bindings
        .iter()
        .map(|b| {
            (
                b["checker"]["path"].as_str().unwrap_or("").to_owned(),
                b["source"].as_str().unwrap_or("").to_owned(),
                b["checker"]["tests"].clone(),
            )
        })
        .collect();
    let derived: Vec<_> = expected
        .iter()
        .map(|(path, source, tests)| (path.clone(), source.clone(), serde_json::json!(tests)))
        .collect();
    // Ordered equality of (checker, input, exact tests) rejects missing, extra,
    // duplicate and unrelated bindings, including a test bound to another row's input.
    require!(bound == derived, "operation checker inventory mismatch");
    for binding in bindings {
        let checker = &binding["checker"];
        let path = checker["path"].as_str().ok_or("missing checker path")?;
        let bytes = files.get(path).ok_or("missing checker source")?;
        require!(
            checker["version"] == "f09-operation-oracle-v1",
            "checker version drift"
        );
        require!(
            checker["sha256"] == sha(bytes),
            "checker source digest drift"
        );
        let text = std::str::from_utf8(bytes).map_err(|_| "checker source encoding")?;
        let helper_records = checker_helpers(text, files)?;
        if helper_records.as_array().is_some_and(Vec::is_empty) {
            require!(
                checker.get("helpers").is_none(),
                "unexpected shared helper binding"
            );
        } else {
            require!(
                checker["helpers"] == helper_records,
                "shared helper binding or digest drift"
            );
        }
        let executable = tests(text);
        require!(!executable.is_empty(), "missing executable checker test");
        let bound_tests = checker["tests"].as_array().ok_or("missing tests")?;
        require!(
            bound_tests
                .iter()
                .all(|t| executable.iter().any(|e| t == e.as_str())),
            "exact executable test binding drift"
        );
        let expected_case = if path.starts_with("xtask/tests/f02_") {
            "F-02"
        } else {
            "F-07"
        };
        require!(binding["case"] == expected_case, "case binding drift");
        let source = binding["source"].as_str().ok_or("missing source")?;
        require!(files.contains_key(source), "missing frozen source bytes");
        let prefix = format!("{}/", source.rsplit_once('/').unwrap().0);
        let expected: Vec<_> = files
            .keys()
            .filter(|p| p.starts_with(&prefix))
            .cloned()
            .collect();
        require!(
            binding["artifacts"] == serde_json::json!(expected),
            "fixture artifact set drift"
        );
        require!(
            expected.iter().all(|p| artifacts.contains_key(p)),
            "missing artifact provenance"
        );
    }
    let actual: Vec<_> = files
        .keys()
        .filter(|p| !is_oracle(p) && !is_helper(p))
        .cloned()
        .collect();
    require!(
        artifacts.keys().cloned().collect::<Vec<_>>() == actual,
        "global artifact inventory drift"
    );
    for (path, record) in artifacts {
        let bytes = files.get(path).ok_or("missing artifact")?;
        require!(
            record["length_bytes"].as_u64() == Some(bytes.len() as u64),
            "artifact byte length drift"
        );
        require!(record["sha256"] == sha(bytes), "artifact digest drift");
    }
    Ok(())
}

#[test]
fn all_operation_oracles_have_frozen_provenance_and_exact_test_bindings() {
    let bytes = fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(MANIFEST)).unwrap();
    assert_eq!(sha(&bytes), MANIFEST_SHA256, "reviewed inventory drift");
    validate(&manifest(), &snapshot()).unwrap();
}

#[test]
fn rejects_missing_binding_and_checker_fixture_artifact_drift() {
    let original = manifest();
    let files = snapshot();
    validate(&original, &files).unwrap();
    let mut missing = original.clone();
    missing["bindings"].as_array_mut().unwrap().remove(0);
    assert!(validate(&missing, &files).is_err());
    for field in ["version", "sha256", "path", "tests"] {
        let mut changed = original.clone();
        changed["bindings"][0]["checker"][field] = Value::Null;
        assert!(
            validate(&changed, &files).is_err(),
            "accepted checker {field} drift"
        );
    }
    let mut changed = original.clone();
    changed["bindings"][0]["source"] = original["bindings"][1]["source"].clone();
    assert!(
        validate(&changed, &files).is_err(),
        "accepted wrong source binding"
    );
    let mut changed = original.clone();
    changed["bindings"][0]["checker"]["tests"] =
        original["bindings"][1]["checker"]["tests"].clone();
    assert!(
        validate(&changed, &files).is_err(),
        "accepted unrelated executable test"
    );
    for field in ["case", "source", "artifacts"] {
        let mut changed = original.clone();
        changed["bindings"][0][field] = Value::Null;
        assert!(
            validate(&changed, &files).is_err(),
            "accepted missing {field}"
        );
    }
    let name_binding = original["bindings"]
        .as_array()
        .unwrap()
        .iter()
        .position(|b| b["checker"]["path"] == "xtask/tests/f02_track_name_edit_oracle.rs")
        .unwrap();
    for field in ["path", "sha256"] {
        let mut changed = original.clone();
        changed["bindings"][name_binding]["checker"]["helpers"][0][field] = Value::Null;
        assert!(
            validate(&changed, &files).is_err(),
            "accepted missing helper {field}"
        );
    }
    let mut changed = original.clone();
    changed["bindings"][name_binding]["checker"]["helpers"][0]["path"] = "wrong.rs".into();
    assert!(
        validate(&changed, &files).is_err(),
        "accepted wrong helper path"
    );
    let mut changed = original.clone();
    changed["bindings"][0]["checker"]["helpers"] = serde_json::json!([]);
    assert!(
        validate(&changed, &files).is_err(),
        "accepted dropped shared decoder helper binding"
    );
    let mut changed = files.clone();
    changed.get_mut(XML_TREE_HELPER).unwrap()[0] ^= 1;
    assert!(
        validate(&original, &changed).is_err(),
        "accepted same-length shared decoder helper drift"
    );
    // Table-driven TRACK attribute checker: each row's test is bound to exactly
    // that row's frozen input; swapping inputs or tests between rows is rejected.
    let rows: Vec<_> = original["bindings"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, b)| b["checker"]["path"] == TRACK_ATTRIBUTE_ORACLE)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(rows.len(), 25, "TRACK attribute table row bindings");
    let (a, b) = (rows[0], rows[1]);
    let mut changed = original.clone();
    changed["bindings"][a]["source"] = original["bindings"][b]["source"].clone();
    changed["bindings"][a]["artifacts"] = original["bindings"][b]["artifacts"].clone();
    assert!(
        validate(&changed, &files).is_err(),
        "accepted table row bound to another row's input"
    );
    let mut changed = original.clone();
    changed["bindings"][a]["checker"]["tests"] =
        original["bindings"][b]["checker"]["tests"].clone();
    assert!(
        validate(&changed, &files).is_err(),
        "accepted table row bound to another row's test"
    );
    let mut changed = files.clone();
    changed.get_mut(TRACK_NAME_HELPER).unwrap()[0] ^= 1;
    assert!(
        validate(&original, &changed).is_err(),
        "accepted same-length helper drift"
    );
    let mut changed = files.clone();
    changed.remove(TRACK_NAME_HELPER);
    assert!(
        validate(&original, &changed).is_err(),
        "accepted missing helper source"
    );
    for key in files.keys() {
        let mut changed = files.clone();
        changed.get_mut(key).unwrap()[0] ^= 1;
        assert!(
            validate(&original, &changed).is_err(),
            "accepted same-length byte drift: {key}"
        );
        changed.get_mut(key).unwrap().push(b' ');
        assert!(
            validate(&original, &changed).is_err(),
            "accepted byte drift: {key}"
        );
        changed.remove(key);
        assert!(
            validate(&original, &changed).is_err(),
            "accepted missing file: {key}"
        );
    }
    let mut changed = files.clone();
    changed.insert(
        "xtask/tests/f02_unbound_oracle.rs".into(),
        b"#[test]\nfn unbound() {}".to_vec(),
    );
    assert!(
        validate(&original, &changed).is_err(),
        "accepted new unbound oracle"
    );
    let first = original["bindings"][0]["source"].as_str().unwrap();
    let extra = format!("{}/unexpected.json", first.rsplit_once('/').unwrap().0);
    let mut changed = files.clone();
    changed.insert(extra, b"{}".to_vec());
    assert!(
        validate(&original, &changed).is_err(),
        "accepted extra fixture artifact"
    );
    let mut changed = original;
    let duplicate = changed["bindings"][0].clone();
    changed["bindings"].as_array_mut().unwrap().push(duplicate);
    assert!(
        validate(&changed, &files).is_err(),
        "accepted duplicate binding"
    );
}
