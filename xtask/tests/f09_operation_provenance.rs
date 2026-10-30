//! F-09 provenance for bounded synthetic F-02/F-07 operation oracles, not qualification.
use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

type Files = BTreeMap<String, Vec<u8>>;
const MANIFEST: &str = "tests/fixtures/f09-operation-provenance.json";
const MANIFEST_SHA256: &str = "192a3155bfe5c4fe0cd19a3fb3652c346a2b158620d18b9cdd16fb9824b282f9";

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
fn fixture(source: &str) -> String {
    let relative = source
        .split("include_bytes!(\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    format!("xtask/tests/{relative}")
}
fn tests(source: &str) -> Vec<String> {
    source
        .split("#[test]\nfn ")
        .skip(1)
        .map(|s| s.split('(').next().unwrap().to_owned())
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
        assert!(!tests(std::str::from_utf8(&bytes).unwrap()).is_empty());
        let source = fixture(std::str::from_utf8(&bytes).unwrap());
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
        files.insert(key, bytes);
    }
    files
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
    let discovered: Vec<_> = files.keys().filter(|p| is_oracle(p)).cloned().collect();
    let bound: Vec<_> = bindings
        .iter()
        .map(|b| b["checker"]["path"].as_str().unwrap_or("").to_owned())
        .collect();
    // Sorted equality rejects missing, extra, duplicate and unrelated bindings.
    require!(bound == discovered, "operation checker inventory mismatch");
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
        let executable = tests(text);
        require!(!executable.is_empty(), "missing executable checker test");
        require!(
            checker["tests"] == serde_json::json!(executable),
            "exact executable test binding drift"
        );
        let expected_case = if path.starts_with("xtask/tests/f02_") {
            "F-02"
        } else {
            "F-07"
        };
        require!(binding["case"] == expected_case, "case binding drift");
        let source = fixture(text);
        require!(
            binding["source"] == source,
            "checker input fixture binding drift"
        );
        require!(files.contains_key(&source), "missing frozen source bytes");
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
    let actual: Vec<_> = files.keys().filter(|p| !is_oracle(p)).cloned().collect();
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
