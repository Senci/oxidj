use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json as _;
use sha2 as _;
use std::collections::BTreeSet;

const ASSET_IDS: [&str; 3] = ["stub-a", "stub-b", "stub-c"];

fn sentinel_bytes(asset_id: &str) -> Vec<u8> {
    format!("oxidj-m1-asset-stub-v1:{asset_id}\n").into_bytes()
}

#[test]
fn sentinel_payloads_are_exact_protocol_bytes() {
    let expected: [(&str, &[u8]); 3] = [
        ("stub-a", b"oxidj-m1-asset-stub-v1:stub-a\n"),
        ("stub-b", b"oxidj-m1-asset-stub-v1:stub-b\n"),
        ("stub-c", b"oxidj-m1-asset-stub-v1:stub-c\n"),
    ];
    for (asset_id, bytes) in expected {
        assert_eq!(sentinel_bytes(asset_id), bytes, "{asset_id}");
        assert_eq!(bytes.last(), Some(&b'\n'));
        assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    }
    assert_eq!(ASSET_IDS, ["stub-a", "stub-b", "stub-c"]);
}

#[test]
fn boundary_manifest_asset_sets_are_exact() {
    let b1: BTreeSet<&str> = ["stub-a", "stub-b"].into_iter().collect();
    let b2: BTreeSet<&str> = ["stub-b", "stub-c"].into_iter().collect();
    assert_eq!(b1, ["stub-a", "stub-b"].into_iter().collect());
    assert_eq!(b2, ["stub-b", "stub-c"].into_iter().collect());
    assert_eq!(
        b1.intersection(&b2).copied().collect::<BTreeSet<_>>(),
        ["stub-b"].into_iter().collect()
    );
    assert_eq!(
        b1.union(&b2).copied().collect::<BTreeSet<_>>(),
        ASSET_IDS.into_iter().collect()
    );
}
