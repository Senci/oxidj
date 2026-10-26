//! `cargo xtask` — OxiDJ repo-owned task runner (build-time tool, not a product crate).
//!
//! This binary centralises every command the quality gate and CI invoke, so CI
//! reproduces a worker's handoff exactly: "CI calls the same repo-owned commands"
//! (M0-10). The fast per-handoff gate is [`cmd_ci`] (Tier 1); CI also runs
//! [`cmd_test`] (Tier 2) and [`cmd_deny`] (Tier 3 licence gate, ADR-009).
//!
//! The dependency-layer guard ([`cmd_layers`]) enforces ADR-004's crate
//! boundaries: a lower layer (closer to the domain core) may not depend on a
//! higher one, and core/CLI may not reference UI/audio/persistence/vendor
//! crates. It is demonstrated (see `docs/layers-policy.md`) to FAIL on a planted
//! forbidden edge and PASS once it is removed.
//!
//! Tier policy and exact tool commands: `docs/handoff-checklist.md`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

#[cfg(test)]
use quick_xml as _;
#[cfg(test)]
use sha2 as _;

// --- Layer table (ADR-004) -------------------------------------------------
// Lower layer number = closer to the domain core. A crate may depend only on
// crates at the SAME layer or LOWER; depending on a higher layer is a forbidden
// (upward) edge. `None` = exempt (build tooling, e.g. xtask itself).
//
// Every new product crate added in M1+ MUST be assigned a layer here, otherwise
// `layers` fails with "unclassified crate" — which is the point: the gate stays
// meaningful as the crate set grows.
fn layer_of(name: &str) -> Option<u8> {
    match name {
        "oxidj-core" => Some(0),
        "oxidj-library" => Some(1),
        "oxidj-rekordbox" => Some(2),
        "oxidj-application" => Some(3),
        "oxidj-cli" => Some(4),
        _ => None,
    }
}

/// Build-tooling crates excluded from the layer + denylist rules.
const EXEMPT_CRATES: &[&str] = &["xtask"];

// --- Infrastructure denylists (ADR-004 boundary rules) ---------------------
// core and CLI: zero UI / audio / persistence / vendor deps by design.
// library: owns accepted local persistence and only the M2 WAV/FLAC/MP3
// decoders; other audio, UI, and vendor adapters remain forbidden.
// A crate on any of these lists in a layered crate's dependencies is a hard
// policy failure, regardless of dependency KIND (normal/dev/build), so the gate
// also catches forbidden dev-dependencies.
const AUDIO_CRATES: &[&str] = &[
    "cpal",
    "symphonia",
    "rodio",
    "rubato",
    "dasp",
    "fundsp",
    "minimp3",
    "hound",
    "claxon",
    "nanomp3",
    "nanomp3-core",
    "lewton",
    "audiobuffer",
    "wavr",
    "soundpipe",
];
const UI_CRATES: &[&str] = &[
    "egui",
    "eframe",
    "slint",
    "iced",
    "tauri",
    "druid",
    "fltk",
    "gtk",
    "gtk4",
    "webkit2gtk",
    "wry",
];
const PERSISTENCE_CRATES: &[&str] = &[
    "rusqlite",
    "rusqlite_migration",
    "sqlx",
    "libsql",
    "diesel",
    "duckdb",
    "sled",
    "redb",
    "sqlite",
    "rocksdb",
    "lmdb",
    "heed",
];
const VENDOR_CRATES: &[&str] = &[
    "rekordbox",
    "serato",
    "mixxx",
    "onelibrary",
    "crate-digger",
    "djay",
    "pyrekordbox",
];

/// Infrastructure category a crate name belongs to, if any.
fn denied_category(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    if AUDIO_CRATES.contains(&lower.as_str()) {
        Some("audio")
    } else if UI_CRATES.contains(&lower.as_str()) {
        Some("ui")
    } else if PERSISTENCE_CRATES.contains(&lower.as_str()) {
        Some("persistence")
    } else if VENDOR_CRATES.contains(&lower.as_str()) {
        Some("vendor")
    } else {
        None
    }
}

fn workspace_root() -> PathBuf {
    // `cargo xtask` is invoked as `cargo run --package xtask -- <args>`, which
    // sets CARGO_MANIFEST_DIR to the xtask package directory (<root>/xtask).
    // The workspace root is its parent. Fall back to "." for direct binary use.
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn cargo_metadata() -> Result<serde_json::Value, String> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(workspace_root())
        .output()
        .map_err(|e| format!("could not spawn `cargo metadata`: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "`cargo metadata --no-deps` failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("`cargo metadata` output was not valid JSON: {e}"))
}

struct DepInfo {
    name: String,
    source: Option<String>,
    kind: Option<String>,
    req: String,
    default_features: bool,
    features: Vec<String>,
}

fn parse_deps(array: &serde_json::Value) -> Vec<DepInfo> {
    let Some(list) = array.as_array() else {
        return Vec::new();
    };
    list.iter()
        .map(|d| DepInfo {
            name: d["name"].as_str().unwrap_or_default().to_string(),
            source: d["source"].as_str().map(String::from),
            kind: d["kind"].as_str().map(String::from),
            req: d["req"].as_str().unwrap_or_default().to_string(),
            default_features: d["uses_default_features"].as_bool().unwrap_or(true),
            features: d["features"]
                .as_array()
                .map(|features| {
                    features
                        .iter()
                        .filter_map(|feature| feature.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

/// Check the workspace dependency graph against ADR-004's layer rules.
/// Returns `Ok(())` when clean, or `Err(violation_messages)` otherwise.
fn check_layers(meta: &serde_json::Value) -> Result<(), Vec<String>> {
    let Some(packages) = meta["packages"].as_array() else {
        return Err(vec!["cargo metadata: missing `packages`".to_string()]);
    };
    let member_ids: Vec<String> = meta["workspace_members"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // (name, layer, deps) for each workspace member.
    let mut members: Vec<(String, Option<u8>, Vec<DepInfo>)> = Vec::new();
    for pkg in packages {
        let id = pkg["id"].as_str().unwrap_or("").to_string();
        if !member_ids.contains(&id) {
            continue;
        }
        let name = pkg["name"].as_str().unwrap_or("").to_string();
        let layer = layer_of(&name);
        let deps = parse_deps(&pkg["dependencies"]);
        members.push((name, layer, deps));
    }
    if members.is_empty() {
        return Err(vec![
            "no workspace members found in `cargo metadata`".to_string(),
        ]);
    }

    // name -> layer, to classify path deps as internal edges.
    let layer_by_name: HashMap<&str, Option<u8>> = members
        .iter()
        .map(|(name, layer, _)| (name.as_str(), *layer))
        .collect();

    let mut violations = Vec::new();
    for (name, layer, deps) in &members {
        let Some(my_layer) = layer else {
            if EXEMPT_CRATES.contains(&name.as_str()) {
                continue;
            }
            violations.push(format!(
                "{name} is a workspace member with no assigned layer; add it to the \
                 layer table in xtask/src/main.rs (ADR-004)"
            ));
            continue;
        };

        for dep in deps {
            let dep_name = &dep.name;
            let is_path_dep = dep.source.is_none();
            if is_path_dep {
                // A path dependency is an internal edge to a workspace member
                // (identified by its package name). Same-or-lower layer: allowed.
                if let Some(&target) = layer_by_name.get(dep_name.as_str()) {
                    if let Some(target_layer) = target
                        && target_layer > *my_layer
                    {
                        violations.push(format!(
                            "{name} (layer {my_layer}) -> {dep_name} (layer {target_layer}): \
                             upward dependency edge is forbidden by ADR-004"
                        ));
                    }
                    continue;
                }
                // A path dep to a non-member is treated as external below.
            }

            // External dependency (registry / git / path-to-non-member).
            let accepted_library_persistence = name == "oxidj-library"
                && matches!(dep_name.as_str(), "rusqlite" | "rusqlite_migration");
            let expected_audio_req = match dep_name.as_str() {
                "hound" => Some("=3.5.1"),
                "claxon" => Some("=0.4.3"),
                "nanomp3" => Some("=0.2.0"),
                _ => None,
            };
            let accepted_library_audio = name == "oxidj-library"
                && expected_audio_req == Some(dep.req.as_str())
                && dep.kind.is_none()
                && !dep.default_features
                && dep.features.is_empty();
            if !accepted_library_persistence
                && !accepted_library_audio
                && let Some(category) = denied_category(dep_name)
            {
                violations.push(format!(
                    "{name} (layer {my_layer}) declares forbidden {category} dependency \
                     '{dep_name}' — ADR-004: {name} must not reference {category} infrastructure",
                ));
            }
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

// ---------------------------------------------------------------------------
// Gate steps
// ---------------------------------------------------------------------------

/// Run a `cargo` subcommand that *is* the repository's own gate step, streaming
/// its output and returning its exit code (0 = pass).
fn run_cargo_step(step: &str, args: &[&str], env_extra: &[(&str, &str)]) -> i32 {
    println!();
    println!("==> [{step}] cargo {}", args.join(" "));
    let mut cmd = Command::new("cargo");
    cmd.args(args)
        .current_dir(workspace_root())
        .env("CARGO_TERM_COLOR", "never");
    for (key, value) in env_extra {
        cmd.env(key, value);
    }
    let exit_code = match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            println!("==> [{step}] FAIL: could not spawn cargo: {e}");
            return 1;
        }
    };
    if exit_code == 0 {
        println!("==> [{step}] PASS (exit 0)");
    } else {
        println!("==> [{step}] FAIL (exit {exit_code})");
    }
    exit_code
}

fn cmd_layers() -> i32 {
    println!();
    println!("==> [layers] cargo xtask layers — ADR-004 dependency guard");
    let meta = match cargo_metadata() {
        Ok(m) => m,
        Err(e) => {
            println!("==> [layers] FAIL");
            println!("{e}");
            return 1;
        }
    };
    match check_layers(&meta) {
        Ok(()) => {
            println!("==> [layers] PASS — all dependency edges within allowed layers");
            0
        }
        Err(violations) => {
            for line in &violations {
                println!("==> [layers] FAIL — {line}");
            }
            1
        }
    }
}

fn cmd_fmt() -> i32 {
    run_cargo_step("fmt", &["fmt", "--all", "--", "--check"], &[])
}

fn cmd_clippy() -> i32 {
    run_cargo_step(
        "clippy",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--keep-going",
            "--locked",
        ],
        &[("CARGO_BUILD_WARNINGS", "deny")],
    )
}

fn cmd_check() -> i32 {
    run_cargo_step(
        "check",
        &[
            "check",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
        ],
        &[],
    )
}

fn cmd_test() -> i32 {
    run_cargo_step(
        "test",
        &["test", "--workspace", "--all-features", "--locked"],
        &[],
    )
}

fn cmd_deny() -> i32 {
    run_cargo_step("deny", &["deny", "check", "licenses"], &[])
}

/// Tier 1 — the per-handoff gate. Stable-only and fast; no remote advisory scans (Cargo may fetch a cold cache).
fn cmd_ci() -> i32 {
    println!("cargo xtask ci — Tier 1 per-handoff gate (stable 1.99.0)");
    if let Some(failed) = stop_on_fail("layers", cmd_layers()) {
        return failed;
    }
    if let Some(failed) = stop_on_fail("fmt", cmd_fmt()) {
        return failed;
    }
    if let Some(failed) = stop_on_fail("clippy", cmd_clippy()) {
        return failed;
    }
    if let Some(failed) = stop_on_fail("check", cmd_check()) {
        return failed;
    }
    println!();
    println!("=> ci summary: layers=ok fmt=ok clippy=ok check=ok  [all pass]");
    0
}

/// Full CI gate: Tier 1 + Tier-2 tests + mandatory Tier-2 licence gate (ADR-009).
fn cmd_ci_full() -> i32 {
    println!("cargo xtask ci-full — CI gate (Tier 1 + tests + licence)");
    if let Some(failed) = stop_on_fail("ci", cmd_ci()) {
        return failed;
    }
    if let Some(failed) = stop_on_fail("test", cmd_test()) {
        return failed;
    }
    if let Some(failed) = stop_on_fail("deny", cmd_deny()) {
        return failed;
    }
    println!();
    println!("=> ci-full summary: ci=ok test=ok deny=ok  [all pass]");
    0
}

/// Returns `Some(exit)` (and logs the failed step) if the step failed.
fn stop_on_fail(step: &str, code: i32) -> Option<i32> {
    if code == 0 {
        None
    } else {
        println!("=> stopping: step '{step}' failed (exit {code})");
        Some(code)
    }
}

fn show_help() -> i32 {
    println!("cargo xtask — OxiDJ repo-owned task runner");
    println!();
    println!("Tier 1 (per handoff, stable, fast; no advisory scans):");
    println!("  ci         layers + fmt + clippy + check");
    println!("  layers     ADR-004 dependency-layer guard");
    println!("  fmt        cargo fmt --all -- --check");
    println!(
        "  clippy     cargo clippy --workspace --all-targets --all-features --keep-going --locked"
    );
    println!("  check      cargo check --workspace --all-targets --all-features --locked");
    println!();
    println!("Tier 2 (CI/full gate):");
    println!("  test       cargo test --workspace --all-features --locked");
    println!("  deny       cargo deny check licenses (ADR-009 MIT allowlist)");
    println!("  ci-full    ci + test + deny");
    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let exit_code = match args.get(1).map(String::as_str) {
        None => show_help(),
        Some("ci") => cmd_ci(),
        Some("ci-full") => cmd_ci_full(),
        Some("layers") => cmd_layers(),
        Some("fmt") => cmd_fmt(),
        Some("clippy") => cmd_clippy(),
        Some("check") => cmd_check(),
        Some("test") => cmd_test(),
        Some("deny") => cmd_deny(),
        Some("help") | Some("-h") | Some("--help") => show_help(),
        Some(unknown) => {
            println!("unknown command: {unknown}");
            println!();
            show_help();
            2
        }
    };
    std::process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn layers_assign_core_bottom_product_crates_above_xtask_exempt() {
        assert_eq!(layer_of("oxidj-core"), Some(0));
        assert_eq!(layer_of("oxidj-library"), Some(1));
        assert_eq!(layer_of("oxidj-rekordbox"), Some(2));
        assert_eq!(layer_of("oxidj-application"), Some(3));
        assert_eq!(layer_of("oxidj-cli"), Some(4));
        assert_eq!(layer_of("xtask"), None);
    }

    #[test]
    fn denylist_covers_each_category() {
        assert_eq!(denied_category("cpal"), Some("audio"));
        assert_eq!(denied_category("egui"), Some("ui"));
        assert_eq!(denied_category("rusqlite"), Some("persistence"));
        assert_eq!(denied_category("serato"), Some("vendor"));
        assert_eq!(denied_category("serde_json"), None);
    }

    fn member(name: &str, deps: Vec<(&str, Option<&str>)>) -> serde_json::Value {
        let deps_val = json!(
            deps.iter()
                .map(|(n, src)| {
                    let (req, uses_default_features) = match *n {
                        "hound" => ("=3.5.1", false),
                        "claxon" => ("=0.4.3", false),
                        "nanomp3" => ("=0.2.0", false),
                        _ => ("*", true),
                    };
                    json!({
                        "name": n,
                        "source": src,
                        "kind": null,
                        "req": req,
                        "uses_default_features": uses_default_features,
                        "features": []
                    })
                })
                .collect::<Vec<_>>()
        );
        json!({
            "name": name,
            "id": format!("path+file:///root#{name}@0.1.0"),
            "dependencies": deps_val
        })
    }

    #[test]
    fn clean_workspace_has_no_violations() {
        let meta = json!({
            "workspace_members": [
                "path+file:///root#oxidj-core@0.1.0",
                "path+file:///root#oxidj-cli@0.1.0"
            ],
            "packages": [
                member("oxidj-core", vec![]),
                member("oxidj-cli", vec![("oxidj-core", None)]),
            ]
        });
        assert!(check_layers(&meta).is_ok());
    }

    #[test]
    fn library_allows_only_accepted_persistence_crates() {
        let meta = json!({
            "workspace_members": [
                "path+file:///root#oxidj-core@0.1.0",
                "path+file:///root#oxidj-library@0.1.0",
                "path+file:///root#oxidj-cli@0.1.0"
            ],
            "packages": [
                member("oxidj-core", vec![]),
                member("oxidj-library", vec![
                    ("oxidj-core", None),
                    ("rusqlite", Some("registry+https://index.crates.io/")),
                    ("rusqlite_migration", Some("registry+https://index.crates.io/")),
                    ("hound", Some("registry+https://index.crates.io/")),
                    ("claxon", Some("registry+https://index.crates.io/")),
                    ("nanomp3", Some("registry+https://index.crates.io/")),
                ]),
                member("oxidj-cli", vec![("oxidj-library", None)]),
            ]
        });
        assert!(check_layers(&meta).is_ok());

        let forbidden = json!({
            "workspace_members": ["path+file:///root#oxidj-library@0.1.0"],
            "packages": [member("oxidj-library", vec![
                ("sqlx", Some("registry+https://index.crates.io/")),
                ("rodio", Some("registry+https://index.crates.io/")),
            ])]
        });
        let errors = check_layers(&forbidden).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| { error.contains("forbidden persistence dependency 'sqlx'") })
        );
        assert!(
            errors
                .iter()
                .any(|error| { error.contains("forbidden audio dependency 'rodio'") })
        );
    }

    #[test]
    fn m2_decoders_are_library_only() {
        for decoder in ["hound", "claxon", "nanomp3"] {
            let library = json!({
                "workspace_members": ["path+file:///root#oxidj-library@0.1.0"],
                "packages": [member("oxidj-library", vec![(
                    decoder,
                    Some("registry+https://index.crates.io/")
                )])]
            });
            assert!(
                check_layers(&library).is_ok(),
                "oxidj-library should allow {decoder}"
            );

            let core = json!({
                "workspace_members": ["path+file:///root#oxidj-core@0.1.0"],
                "packages": [member("oxidj-core", vec![(
                    decoder,
                    Some("registry+https://index.crates.io/")
                )])]
            });
            let errors = check_layers(&core).unwrap_err();
            assert!(errors.iter().any(|error| {
                error.contains(&format!("forbidden audio dependency '{decoder}'"))
            }));
        }
    }

    #[test]
    fn decoder_exception_requires_exact_normal_dependency_configuration() {
        for (field, invalid_value) in [
            ("req", json!("^3.5")),
            ("uses_default_features", json!(true)),
            ("kind", json!("dev")),
            ("features", json!(["std"])),
        ] {
            let mut meta = json!({
                "workspace_members": ["path+file:///root#oxidj-library@0.1.0"],
                "packages": [member("oxidj-library", vec![(
                    "hound",
                    Some("registry+https://index.crates.io/")
                )])]
            });
            meta["packages"][0]["dependencies"][0][field] = invalid_value;
            let errors = check_layers(&meta).unwrap_err();
            assert!(
                errors
                    .iter()
                    .any(|error| { error.contains("forbidden audio dependency 'hound'") })
            );
        }
    }

    #[test]
    fn library_rejects_unapproved_audio_crates() {
        for dependency in ["symphonia", "nanomp3-core", "minimp3", "rodio"] {
            let meta = json!({
                "workspace_members": ["path+file:///root#oxidj-library@0.1.0"],
                "packages": [member("oxidj-library", vec![(
                    dependency,
                    Some("registry+https://index.crates.io/")
                )])]
            });
            let errors = check_layers(&meta).unwrap_err();
            assert!(errors.iter().any(|error| {
                error.contains(&format!("forbidden audio dependency '{dependency}'"))
            }));
        }
    }

    #[test]
    fn cli_cannot_depend_on_persistence_crates() {
        let meta = json!({
            "workspace_members": ["path+file:///root#oxidj-cli@0.1.0"],
            "packages": [member("oxidj-cli", vec![(
                "rusqlite", Some("registry+https://index.crates.io/")
            )])]
        });
        let errors = check_layers(&meta).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("forbidden persistence dependency 'rusqlite'"))
        );
    }

    #[test]
    fn core_and_library_reject_nonaccepted_infrastructure() {
        for (crate_name, dependency, category) in [
            ("oxidj-core", "rusqlite", "persistence"),
            ("oxidj-library", "symphonia", "audio"),
            ("oxidj-library", "eframe", "ui"),
            ("oxidj-library", "rekordbox", "vendor"),
        ] {
            let meta = json!({
                "workspace_members": [format!("path+file:///root#{crate_name}@0.1.0")],
                "packages": [member(crate_name, vec![(
                    dependency,
                    Some("registry+https://index.crates.io/")
                )])]
            });
            let errors = check_layers(&meta).unwrap_err();
            assert!(
                errors.iter().any(|error| error
                    .contains(&format!("forbidden {category} dependency '{dependency}'"))),
                "expected {crate_name} -> {dependency} to be denied, got {errors:?}"
            );
        }
    }

    #[test]
    fn planted_audio_dep_in_core_is_flagged() {
        let meta = json!({
            "workspace_members": [
                "path+file:///root#oxidj-core@0.1.0",
                "path+file:///root#oxidj-cli@0.1.0"
            ],
            "packages": [
                member("oxidj-core", vec![("cpal", Some("registry+https://github.com/rust-lang/crates.io-index"))]),
                member("oxidj-cli", vec![("oxidj-core", None)]),
            ]
        });
        let errs = check_layers(&meta).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.contains("cpal") && e.contains("audio"))
        );
    }

    #[test]
    fn upward_internal_edge_is_forbidden() {
        let meta = json!({
            "workspace_members": [
                "path+file:///root#oxidj-core@0.1.0",
                "path+file:///root#oxidj-cli@0.1.0"
            ],
            "packages": [
                member("oxidj-core", vec![("oxidj-cli", None)]),
                member("oxidj-cli", vec![]),
            ]
        });
        let errs = check_layers(&meta).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("upward")));
    }

    #[test]
    fn unclassified_member_is_flagged() {
        let meta = json!({
            "workspace_members": [
                "path+file:///root#oxidj-core@0.1.0",
                "path+file:///root#mystery-crate@0.1.0"
            ],
            "packages": [
                member("oxidj-core", vec![]),
                member("mystery-crate", vec![]),
            ]
        });
        let errs = check_layers(&meta).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| e.contains("mystery-crate") && e.contains("no assigned layer"))
        );
    }

    #[test]
    fn exempt_xtask_is_skipped() {
        let meta = json!({
            "workspace_members": [
                "path+file:///root#oxidj-core@0.1.0",
                "path+file:///root#oxidj-cli@0.1.0",
                "path+file:///root#xtask@0.1.0"
            ],
            "packages": [
                member("oxidj-core", vec![]),
                member("oxidj-cli", vec![("oxidj-core", None)]),
                // xtask pulls serde_json (external) — must NOT trip the infra denylist.
                member("xtask", vec![("serde_json", Some("registry+https://github.com/rust-lang/crates.io-index"))]),
            ]
        });
        assert!(check_layers(&meta).is_ok());
    }
}
