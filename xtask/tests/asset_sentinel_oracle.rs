use oxidj_rekordbox as _;
use quick_xml as _;
use serde_json as _;
use sha2 as _;
use sha2::{Digest, Sha256};
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

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn manifest(boundary_id: &str, asset_ids: &[&str]) -> (Vec<u8>, String) {
    let entries: Vec<_> = asset_ids
        .iter()
        .map(|asset_id| {
            let payload = sentinel_bytes(asset_id);
            serde_json::json!({
                "asset_id": asset_id,
                "relative_path": format!("assets/{asset_id}.bin"),
                "byte_length": payload.len(),
                "sha256": sha256(&payload),
            })
        })
        .collect();
    let value = serde_json::json!({
        "format_version": 1,
        "fixture_id": "oxidj-m1-asset-stubs-v1",
        "boundary_id": boundary_id,
        "entries": entries,
    });
    let mut bytes = serde_json::to_vec(&value).unwrap();
    bytes.push(b'\n');
    let digest = sha256(&bytes);
    (bytes, digest)
}

fn verify_boundary(
    manifest_bytes: &[u8],
    payloads: &std::collections::BTreeMap<String, Vec<u8>>,
    boundary_id: &str,
) -> bool {
    let (asset_ids, frozen_digest) = match boundary_id {
        "B1" => (
            &["stub-a", "stub-b"][..],
            "7d5ac965ddc066e29e4addbeb9a8f07dc62a8790e0abc2a563566f23f772fc8e",
        ),
        "B2" => (
            &["stub-b", "stub-c"][..],
            "f7efd43d02205d7c64a2ef3c349b9606870b2ceddc8b0e679c99c9fc9a46a9ec",
        ),
        _ => return false,
    };
    let (expected_manifest, _) = manifest(boundary_id, asset_ids);
    if manifest_bytes != expected_manifest || sha256(manifest_bytes) != frozen_digest {
        return false;
    }
    if payloads.len() != asset_ids.len() {
        return false;
    }
    asset_ids.iter().all(|asset_id| {
        payloads
            .get(*asset_id)
            .is_some_and(|actual| actual == &sentinel_bytes(asset_id))
    })
}

type LiveReferences = std::collections::BTreeMap<&'static str, (&'static str, &'static str)>;

fn b1_to_b2(live: &LiveReferences, available_assets: &BTreeSet<&str>) -> Option<LiveReferences> {
    let expected_b1: LiveReferences = [
        ("ref-1", ("asset-track-1", "stub-a")),
        ("ref-2", ("asset-track-2", "stub-b")),
    ]
    .into_iter()
    .collect();
    if live != &expected_b1
        || !["stub-b", "stub-c"]
            .iter()
            .all(|asset_id| available_assets.contains(asset_id))
    {
        return None;
    }

    Some(
        [
            ("ref-1", ("asset-track-1", "stub-b")),
            ("ref-2", ("asset-track-2", "stub-c")),
        ]
        .into_iter()
        .collect(),
    )
}

#[test]
fn b1_to_b2_changes_both_live_bindings_and_manifest_follows_live_state() {
    let b1: LiveReferences = [
        ("ref-1", ("asset-track-1", "stub-a")),
        ("ref-2", ("asset-track-2", "stub-b")),
    ]
    .into_iter()
    .collect();
    let available: BTreeSet<_> = ASSET_IDS.into_iter().collect();
    let b2 = b1_to_b2(&b1, &available).expect("the exact B1 transition is valid");

    assert_eq!(b1["ref-1"].0, b2["ref-1"].0);
    assert_eq!(b1["ref-2"].0, b2["ref-2"].0);
    assert_eq!(b2["ref-1"].1, "stub-b");
    assert_eq!(b2["ref-2"].1, "stub-c");
    let b2_ids: Vec<_> = b2.values().map(|(_, asset)| *asset).collect();
    assert_eq!(
        b2_ids.iter().copied().collect::<BTreeSet<_>>(),
        ["stub-b", "stub-c"].into_iter().collect()
    );
    assert!(!b2.values().any(|(_, asset)| *asset == "stub-a"));
    let (b2_manifest, _) = manifest("B2", &b2_ids);
    let b2_payloads: std::collections::BTreeMap<_, _> = b2_ids
        .iter()
        .map(|asset_id| ((*asset_id).to_owned(), sentinel_bytes(asset_id)))
        .collect();
    assert!(verify_boundary(&b2_manifest, &b2_payloads, "B2"));
}

#[test]
fn b1_to_b2_rejects_stale_or_missing_target_without_partial_transition() {
    let b1: LiveReferences = [
        ("ref-1", ("asset-track-1", "stub-a")),
        ("ref-2", ("asset-track-2", "stub-b")),
    ]
    .into_iter()
    .collect();
    let mut stale = b1.clone();
    stale.insert("ref-1", ("asset-track-1", "stub-c"));
    let all_assets: BTreeSet<_> = ASSET_IDS.into_iter().collect();
    let missing_target: BTreeSet<_> = ["stub-a", "stub-b"].into_iter().collect();

    assert!(b1_to_b2(&stale, &all_assets).is_none());
    assert!(b1_to_b2(&b1, &missing_target).is_none());
    assert_eq!(b1["ref-1"].1, "stub-a");
    assert_eq!(b1["ref-2"].1, "stub-b");
}

#[test]
fn in_memory_boundary_verifier_passes_goldens_and_rejects_integrity_faults() {
    for (boundary, ids) in [
        ("B1", &["stub-a", "stub-b"][..]),
        ("B2", &["stub-b", "stub-c"][..]),
    ] {
        let (golden, _) = manifest(boundary, ids);
        let payloads: std::collections::BTreeMap<_, _> = ids
            .iter()
            .map(|id| ((*id).to_owned(), sentinel_bytes(id)))
            .collect();
        assert!(verify_boundary(&golden, &payloads, boundary));

        let mut missing = payloads.clone();
        missing.remove(ids[0]);
        assert!(
            !verify_boundary(&golden, &missing, boundary),
            "missing listed asset"
        );

        let mut changed = payloads.clone();
        changed.get_mut(ids[0]).unwrap()[0] ^= 1;
        assert!(
            !verify_boundary(&golden, &changed, boundary),
            "changed payload byte"
        );

        let mut tampered = golden.clone();
        tampered[0] ^= 1;
        assert!(
            !verify_boundary(&tampered, &payloads, boundary),
            "tampered manifest"
        );
    }
}

#[test]
fn b1_and_b2_manifest_bytes_and_digests_are_frozen() {
    const B1: &[u8] = br#"{"boundary_id":"B1","entries":[{"asset_id":"stub-a","byte_length":30,"relative_path":"assets/stub-a.bin","sha256":"51116fb222616271217977708b14624881449d7fcd4c9e30fe39f45faf629798"},{"asset_id":"stub-b","byte_length":30,"relative_path":"assets/stub-b.bin","sha256":"9ea6ec1c3dea6600ddf03248fe14b579d3497d18a79f0048cfa5c1d544cd2d35"}],"fixture_id":"oxidj-m1-asset-stubs-v1","format_version":1}
"#;
    const B2: &[u8] = br#"{"boundary_id":"B2","entries":[{"asset_id":"stub-b","byte_length":30,"relative_path":"assets/stub-b.bin","sha256":"9ea6ec1c3dea6600ddf03248fe14b579d3497d18a79f0048cfa5c1d544cd2d35"},{"asset_id":"stub-c","byte_length":30,"relative_path":"assets/stub-c.bin","sha256":"d22732bb5055e125ebcc511b2e553ceb8d1f715d457db7bc2b3f4fe0fb90ba0b"}],"fixture_id":"oxidj-m1-asset-stubs-v1","format_version":1}
"#;

    let (b1, b1_digest) = manifest("B1", &["stub-a", "stub-b"]);
    let (b2, b2_digest) = manifest("B2", &["stub-b", "stub-c"]);
    assert_eq!(b1, B1);
    assert_eq!(b2, B2);
    assert_eq!(
        b1_digest,
        "7d5ac965ddc066e29e4addbeb9a8f07dc62a8790e0abc2a563566f23f772fc8e"
    );
    assert_eq!(
        b2_digest,
        "f7efd43d02205d7c64a2ef3c349b9606870b2ceddc8b0e679c99c9fc9a46a9ec"
    );
    assert_eq!(b1.iter().filter(|&&byte| byte == b'\n').count(), 1);
    assert_eq!(b2.iter().filter(|&&byte| byte == b'\n').count(), 1);
    assert!(!b1.starts_with(&[0xef, 0xbb, 0xbf]));
    assert!(!b2.starts_with(&[0xef, 0xbb, 0xbf]));
}
