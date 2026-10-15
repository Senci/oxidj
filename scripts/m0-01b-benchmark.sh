#!/usr/bin/env bash
# M0-01b free-worker validation harness.
# Runs 5 isolated Rust tasks with real cargo output + per-command timing.
# ALL work happens in this scratch dir, which is OUTSIDE the oxidj git repo.
set -u
B="$(cd "$(dirname "$0")" && pwd)"
cd "$B"

mark(){ echo; echo "======================================= $1"; }
dur(){ python3 -c "print(round($1,2))"; }

mark "TASK 1 — multi-module comprehension (djmeta: lib + bpm + key + cue)"
echo "source tree:"; find t1_modules/src -type f | sort
cd t1_modules
echo "[cargo build]"; t0=$(date +%s.%N); cargo build 2>&1; rc=$?; t1=$(date +%s.%N)
echo "[build exit=$rc wall=$(dur $(echo "$t1-$t0"))s]"
echo "[cargo test]"; t0=$(date +%s.%N); cargo test 2>&1; rc=$?; t1=$(date +%s.%N)
echo "[test exit=$rc wall=$(dur $(echo "$t1-$t0"))s]"
cd "$B"

mark "TASK 2 — type error (broken version MUST fail, then fix + rebuild)"
cd t2_typeerror
echo "[cargo build — EXPECTING TYPE ERROR E0308]"; t0=$(date +%s.%N); cargo build 2>&1; rc=$?; t1=$(date +%s.%N)
echo "[build exit=$rc wall=$(dur $(echo "$t1-$t0"))s]"
cd "$B"

mark "TASK 3 — typed API (typed-beats): build + clippy + test"
cd t3_typedapi
echo "[cargo build]"; t0=$(date +%s.%N); cargo build 2>&1; rc=$?; t1=$(date +%s.%N); echo "[build exit=$rc wall=$(dur $(echo "$t1-$t0"))s]"
echo "[cargo clippy --all-targets]"; t0=$(date +%s.%N); cargo clippy --all-targets 2>&1; rc=$?; t1=$(date +%s.%N); echo "[clippy exit=$rc wall=$(dur $(echo "$t1-$t0"))s]"
echo "[cargo test]"; t0=$(date +%s.%N); cargo test 2>&1; rc=$?; t1=$(date +%s.%N); echo "[test exit=$rc wall=$(dur $(echo "$t1-$t0"))s]"
cd "$B"

mark "TASK 4 — test suite: unit + integration + doctests"
cd t4_tests
echo "[cargo test]"; t0=$(date +%s.%N); cargo test 2>&1; rc=$?; t1=$(date +%s.%N); echo "[test exit=$rc wall=$(dur $(echo "$t1-$t0"))s]"
cd "$B"

mark "TASK 5 — patch review: git baseline (bug introduced in a later step)"
cd t5_patchreview
git init -q
git config user.email "bench@oxidj.local"
git config user.name "oxidj-bench"
git add -A
git commit -q -m "baseline: correct inclusive BpmRange"
echo "[git log]"; git --no-pager log --oneline
echo "[cargo test baseline]"; t0=$(date +%s.%N); cargo test 2>&1; rc=$?; t1=$(date +%s.%N); echo "[test exit=$rc wall=$(dur $(echo "$t1-$t0"))s]"
cd "$B"

mark "ALL CORE TASKS DONE"
