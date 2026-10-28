use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

const ROOT: &str = "tests/fixtures/b1-b2-logical";
const CHECKER_VERSION: &str = "b1-b2-logical-checker-v1";

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(ROOT)
        .join(name)
}
fn read(name: &str) -> Vec<u8> {
    fs::read(path(name)).unwrap()
}
fn value(name: &str) -> Value {
    serde_json::from_slice(&read(name)).unwrap()
}
fn canonical(value: &Value) -> Vec<u8> {
    fn sort(v: &Value) -> Value {
        match v {
            Value::Object(m) => {
                let mut keys: Vec<_> = m.keys().collect();
                keys.sort();
                let mut o = serde_json::Map::new();
                for k in keys {
                    o.insert(k.clone(), sort(&m[k]));
                }
                Value::Object(o)
            }
            Value::Array(a) => Value::Array(a.iter().map(sort).collect()),
            other => other.clone(),
        }
    }
    let mut b = serde_json::to_vec(&sort(value)).unwrap();
    b.push(b'\n');
    b
}
fn expected(boundary: &str, refs: [&str; 2], prior: Option<&str>) -> Value {
    let history = if let Some(op) = prior {
        vec![
            serde_json::json!({"operation_id":op,"expected_boundary":"B1","revision_before":0,"revision_after":1,"result":"committed","changes":[{"reference_id":"ref-1","track_id":"asset-track-1","from":"stub-a","to":"stub-b"},{"reference_id":"ref-2","track_id":"asset-track-2","from":"stub-b","to":"stub-c"}]}),
        ]
    } else {
        vec![]
    };
    serde_json::json!({"boundary_id":boundary,"contract_version":"logical-v1","fixture_id":"oxidj-m1-asset-stubs-v1","revision":if prior.is_some(){1}else{0},"tracks":[{"track_id":"asset-track-1"},{"track_id":"asset-track-2"}],"live_references":[{"reference_id":"ref-1","track_id":"asset-track-1","asset_id":refs[0]},{"reference_id":"ref-2","track_id":"asset-track-2","asset_id":refs[1]}],"history":history})
}
fn snapshot_matches(boundary: &str, actual: &[u8]) -> bool {
    let (refs, op) = match boundary {
        "B1" => (["stub-a", "stub-b"], None),
        "B2" => (["stub-b", "stub-c"], Some("op-b1-to-b2")),
        _ => return false,
    };
    actual == canonical(&expected(boundary, refs, op))
}

fn verify(boundary: &str) -> bool {
    let (file, ids): (&str, BTreeSet<&str>) = match boundary {
        "B1" => ("B1.json", ["stub-a", "stub-b"].into_iter().collect()),
        "B2" => ("B2.json", ["stub-b", "stub-c"].into_iter().collect()),
        _ => return false,
    };
    let actual = read(file);
    if !snapshot_matches(boundary, &actual) {
        return false;
    }
    let manifest = read(&format!("{boundary}-asset-manifest.json"));
    let manifest_value: Value = match serde_json::from_slice(&manifest) {
        Ok(v) => v,
        Err(_) => return false,
    };
    if manifest != canonical(&manifest_value) || manifest_value["boundary_id"] != boundary {
        return false;
    }
    let entries = match manifest_value["entries"].as_array() {
        Some(v) => v,
        None => return false,
    };
    let manifest_ids: BTreeSet<_> = entries
        .iter()
        .filter_map(|e| e["asset_id"].as_str())
        .collect();
    if manifest_ids != ids || entries.len() != ids.len() {
        return false;
    }
    for e in entries {
        let Some(id) = e["asset_id"].as_str() else {
            return false;
        };
        let Some(rel) = e["relative_path"].as_str() else {
            return false;
        };
        let bytes = match fs::read(path(rel)) {
            Ok(v) => v,
            Err(_) => return false,
        };
        if rel != format!("assets/{id}.bin")
            || e["byte_length"] != bytes.len()
            || e["sha256"] != digest(&bytes)
            || bytes != format!("oxidj-m1-asset-stub-v1:{id}\n").as_bytes()
        {
            return false;
        }
    }
    let witness = value(&format!("{boundary}-witness.json"));
    witness["boundary_id"] == boundary
        && witness["commit_marker"]
            == if boundary == "B1" {
                "committed-B1"
            } else {
                "committed-B2"
            }
        && witness["snapshot_sha256"] == digest(&actual)
        && witness["manifest_sha256"] == digest(&manifest)
}

#[test]
fn frozen_snapshots_and_witnesses_bind_exact_boundary_state() {
    assert!(verify("B1"));
    assert!(verify("B2"));
    let identity = value("checker-identity.json");
    let checker = fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/b1_b2_logical_snapshots.rs"),
    )
    .unwrap();
    assert_eq!(identity["checker_version"], CHECKER_VERSION);
    assert_eq!(
        identity["checker_path"],
        "xtask/tests/b1_b2_logical_snapshots.rs"
    );
    assert_eq!(identity["checker_sha256"], digest(&checker));
    let provenance = value("provenance.json");
    assert_eq!(provenance["checker_identity"], identity);
    let artifacts = provenance["artifacts"].as_object().unwrap();
    for (name, meta) in artifacts {
        let bytes = read(name);
        assert_eq!(meta["byte_length"], bytes.len());
        assert_eq!(meta["sha256"], digest(&bytes));
    }
}

#[test]
fn boundary_endpoint_history_manifest_and_identity_mutations_fail() {
    let b1 = read("B1.json");
    let b2 = read("B2.json");
    assert!(!snapshot_matches("B1", &b2));
    assert!(!snapshot_matches("B2", &b1));
    for (boundary, mutator) in [("B1", "live_references"), ("B2", "history")] {
        let mut changed = value(&format!("{boundary}.json"));
        if mutator == "live_references" {
            changed["live_references"][0]["asset_id"] = Value::String("stub-c".into());
        } else {
            changed["history"][0]["operation_id"] = Value::String("drifted".into());
        }
        assert!(!snapshot_matches(boundary, &canonical(&changed)));
    }
    let identity = value("checker-identity.json");
    let checker = fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/b1_b2_logical_snapshots.rs"),
    )
    .unwrap();
    assert_eq!(identity["checker_sha256"], digest(&checker));
    let mut altered_identity = identity.clone();
    altered_identity["checker_sha256"] = Value::String("0".repeat(64));
    assert_ne!(altered_identity["checker_sha256"], digest(&checker));
    for boundary in ["B1", "B2"] {
        assert!(verify(boundary));
        let mut b = read(&format!("{boundary}.json"));
        b[0] ^= 1;
        assert_ne!(b, read(&format!("{boundary}.json")));
        let mut m = read(&format!("{boundary}-asset-manifest.json"));
        m[0] ^= 1;
        assert_ne!(m, read(&format!("{boundary}-asset-manifest.json")));
    }
    let mut identity = value("checker-identity.json");
    identity["checker_sha256"] = Value::String("0".repeat(64));
    assert_ne!(identity, value("checker-identity.json"));
}
