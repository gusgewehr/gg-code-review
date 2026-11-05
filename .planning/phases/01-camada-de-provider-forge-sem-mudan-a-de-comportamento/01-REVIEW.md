---
phase: 01-camada-de-provider-forge-sem-mudan-a-de-comportamento
reviewed: 2026-11-05T04:16:25Z
depth: quick
iteration: 2
files_reviewed: 3
files_reviewed_list:
  - backend/src/forge/mod.rs
  - backend/src/server/summary.rs
  - backend/src/server/comment_activation_tests.rs
findings:
  critical: 0
  warning: 0
  info: 2
  total: 2
status: issues_found
---

# Phase 1: Code Review Report (iteration 2)

**Reviewed:** 2026-11-05T04:16:25Z
**Depth:** quick
**Files Reviewed:** 3
**Status:** issues_found (Info only; no Critical or Warning)

## Narrative Findings (AI reviewer)

## Summary

This pass re-checks the fixes for CR-01, CR-02 and CR-03 (commits `a5d5f9e`, `f845719`, `afd8b81`). I checked the layouts against rustfmt's rules by hand, because `cargo` and `rustfmt` are not installed. The settings are `max_width = 120`, so `fn_call_width = 72` and `chain_width = 72`. All three fixes look correct. No new Critical or Warning issues. The two earlier Info items still apply.

- **CR-01 (`forge/mod.rs:43-50`): fixed.** The two `assert_eq!` calls are split again. Their arguments are 81 and 92 characters, which is over 72. The file is byte-identical to its content at `83e3580` (blob `50cea93`), which was the layout before `484a69b`.
- **CR-02 (`summary.rs:249-254`): fixed.**
  - `fetch(...)` is split one argument per line. Its arguments are 84 characters, over 72.
  - The inner `format!` arguments are exactly 68 characters, so `format!` stays on one line at indent 8 (87 columns).
  - `forge::current().pr_head_ref(number)` has a single child, so `chain_width` does not apply to it.
  - The root call now spans several lines and ends in `)`. rustfmt treats that as block-like (`is_block_expr` on a multi-line `Call`), so `.status()` and `.map_err(...)?` stay at the statement's indent (4).
  - The codebase already uses this layout, for example `bench/reviewbench.rs:58` and `db/import_sqlite.rs:536`.
  - Putting the right-hand side on the next line would not save any line breaks, so `choose_rhs` keeps it on the `let` line.
- **CR-03 (`comment_activation_tests.rs:104-107`): fixed.**
  - The `command_number` arguments are exactly 72 characters: 11 + 59 + 2 for the `", "` separator. `definitive_tactic` uses `real_total <= limit`, so the arguments stay on one line. This matches the precedent at `review/output_repair_tests.rs:175`, and the pre-phase 73-character version was split.
  - About `chain_width`: `comment(1).command_number(...)` is 99 characters, more than 72. But it is a chain with one child (parent `comment(1)`, child `.command_number(...)`). rustfmt's `format_last_child` uses `shape.width` instead of `chain_width` when `child_count == 1`, so only the available width matters: 111 columns at indent 8, minus the trailing comma. The line is 108 columns, so it fits on one line.
  - The inner `forge::iso8601_to_epoch_ms(...).unwrap()` is 59 characters and also has a single child.
  - The outer `assert_eq!` arguments are about 105 characters, over 72, and its last argument `None` cannot overflow. So the outer call correctly stays split.
- **Other checks.**
  - The quick-mode pattern scan (secrets, `eval`/`exec`, `dbg!`/`todo!`/`println!`, TODO/FIXME) found nothing in the three files.
  - Several lines are longer than 120 characters: `summary.rs:16, 113, 114, 293` and `comment_activation_tests.rs:77, 399, 503`. Each is a long string literal, a `json!` literal or a `matches!` guard that rustfmt does not reflow, so it leaves them alone. The phase diff only renames `github::` to `forge::` inside them, which makes those lines shorter.

The only remaining doubt is that these are hand checks. Run `cargo fmt --all --check` in the Docker build image before pushing.

## Info

### IN-01: `#[allow(unused_imports)]` on the forge glob re-export does nothing useful

**File:** `backend/src/forge/mod.rs:3`
**Issue:** `unused_imports` only fires on a glob when none of its items are used. This glob is used heavily in both test and non-test builds, so the allow never matters. It would hide the warning if the seam later stopped being used.
**Fix:** Remove the attribute. If clippy complains, keep it with a one-line reason.

### IN-02: The executor fingerprint changes because the rename touched files that are hashed into it

**File:** `backend/src/sandbox/mod.rs:24-43` (hashes `review/mod.rs`, `review/dependencies.rs`, `review/frozen.rs`, `review_archive/mod.rs`, `indexer/mod.rs`, all edited in this phase). This file is outside this iteration's scope; the item is carried over from iteration 1.
**Issue:** Output strings are unchanged, but `sandbox::engine_version()` now produces a new value. `sandbox/runner.rs:1164` rejects runners with a mismatched version. A server built from this branch next to a runner image from before this phase (for example the upstream `ghcr.io/bridglabs/opencodereview-runner`) would fail every review. There is still no `changelog.d/` fragment that mentions this.
**Fix:** No code change needed. Note in the phase or changelog that the server and runner images must be rebuilt together.

---

_Reviewed: 2026-11-05T04:16:25Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: quick_
