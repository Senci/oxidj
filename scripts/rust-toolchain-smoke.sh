#!/usr/bin/env bash
# Disposable, dependency-free tooling proof; not an OxiDJ product crate.
set -euo pipefail
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable}"
scratch_root="${TMPDIR:-$HOME/.cache/oxidj}"
mkdir -p "$scratch_root"
probe="$(mktemp -d "$scratch_root/oxidj-rust-smoke.XXXXXX")"
trap 'rm -rf "$probe"' EXIT
export CARGO_TARGET_DIR="$probe/target"
export CARGO_TERM_COLOR=never
unset CARGO_BUILD_TARGET
cd "$probe"
printf 'Probe cwd: %s\n' "$PWD"
rustc -Vv
cargo -V
rustfmt -V
cargo clippy -V
rust-analyzer --version
cargo new --lib --name oxidj_toolchain_probe --vcs none crate
cd crate

expect_failure() {
    local label="$1"; shift
    if "$@"; then
        printf 'ERROR: expected failure: %s\n' "$label" >&2
        exit 1
    else
        printf 'Expected failure observed: %s\n' "$label"
    fi
}

# Real compiler diagnostics, not a fabricated error transcript.
cat > src/lib.rs <<'RS'
pub fn answer() -> u32 { "wrong type" }
RS
expect_failure 'compiler type error' cargo check --offline --message-format=json

cat > src/lib.rs <<'RS'
pub fn answer()->u32{42}
RS
expect_failure 'formatter drift' cargo fmt --all -- --check
cargo fmt --all

cat > src/lib.rs <<'RS'
pub fn answer() -> bool {
    let value = 42;
    value == value
}
RS
expect_failure 'Clippy eq_op' cargo clippy --offline --all-targets -- -D warnings

# Bug reproduction invariant: assert correct behavior, see RED, fix implementation.
cat > src/lib.rs <<'RS'
/// Return the smoke-test constant.
///
/// ```
/// assert_eq!(oxidj_toolchain_probe::answer(), 42);
/// ```
pub fn answer() -> u32 {
    41
}

#[cfg(test)]
mod tests {
    #[test]
    fn answer_is_42() {
        assert_eq!(super::answer(), 42);
    }
}
RS
expect_failure 'regression test RED' cargo test --offline --lib answer_is_42

cat > src/lib.rs <<'RS'
/// Return the smoke-test constant.
///
/// ```
/// assert_eq!(oxidj_toolchain_probe::answer(), 42);
/// ```
pub fn answer() -> u32 {
    42
}

#[cfg(test)]
mod tests {
    #[test]
    fn answer_is_42() {
        assert_eq!(super::answer(), 42);
    }
}
RS
mkdir -p tests
cat > tests/public_api.rs <<'RS'
#[test]
fn public_api_is_42() {
    assert_eq!(oxidj_toolchain_probe::answer(), 42);
}
RS
cargo fmt --all -- --check
cargo check --offline --locked --all-targets
cargo clippy --offline --locked --all-targets -- -D warnings
cargo test --offline --locked
printf 'PASS: compiler, formatter, Clippy, RED/GREEN regression, unit/integration/doctests\n'
