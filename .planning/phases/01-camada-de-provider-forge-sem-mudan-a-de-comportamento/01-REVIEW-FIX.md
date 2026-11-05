---
phase: 01-camada-de-provider-forge-sem-mudan-a-de-comportamento
fixed_at: 2026-11-05T04:14:32Z
review_path: .planning/phases/01-camada-de-provider-forge-sem-mudan-a-de-comportamento/01-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 1: Code Review Fix Report

**Fixed at:** 2026-11-05T04:14:32Z
**Source review:** .planning/phases/01-camada-de-provider-forge-sem-mudan-a-de-comportamento/01-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3 (critical_warning scope; IN-01 and IN-02 not in scope)
- Fixed: 3
- Skipped: 0

Verification: `cargo`/`rustfmt` are not installed locally, so the layouts were applied by hand
following rustfmt rules (`max_width = 120`, `fn_call_width = 72`, single-child chains exempt from
`chain_width`). Run `cargo fmt --all --check` in the Docker build image to confirm before pushing.

## Fixed Issues

### CR-01: Collapsed `assert_eq!` lines in the forge test fail `cargo fmt --check`

**Files modified:** `backend/src/forge/mod.rs`
**Commit:** a5d5f9e
**Applied fix:** Restored the one-argument-per-line layout for the `commit_url` and `review_url`
`assert_eq!` calls (args 81 and 92 chars, over 72). The file is now byte-identical to its state
before `484a69b` (blob `50cea93`).

### CR-02: `fetch(...)` in `fetch_summary_refs` now needs one argument per line

**Files modified:** `backend/src/server/summary.rs`
**Commit:** f845719
**Applied fix:** Split the `fetch(...)` call (args 84 chars) one argument per line with a trailing
comma; the `.status()` / `.map_err(...)?` chain now hangs off the closing `)` at the statement's
indent, as rustfmt does for a multi-line call root.

### CR-03: The rename shortened a split call to exactly 72 characters, so rustfmt now joins it

**Files modified:** `backend/src/server/comment_activation_tests.rs`
**Commit:** afd8b81
**Applied fix:** Joined the inner `command_number("acme/repo", forge::iso8601_to_epoch_ms(...).unwrap())`
call onto one line (args exactly 72 chars, line 108 chars). The outer `assert_eq!` stays split.

---

_Fixed: 2026-11-05T04:14:32Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
