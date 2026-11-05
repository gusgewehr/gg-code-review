---
phase: 01-camada-de-provider-forge-sem-mudan-a-de-comportamento
reviewed: 2026-11-05T04:11:56Z
depth: standard
files_reviewed: 30
files_reviewed_list:
  - backend/src/db/review_requests.rs
  - backend/src/forge/mod.rs
  - backend/src/indexer/mod.rs
  - backend/src/lib.rs
  - backend/src/review/dependencies.rs
  - backend/src/review/execution.rs
  - backend/src/review/frozen.rs
  - backend/src/review/git.rs
  - backend/src/review/mod.rs
  - backend/src/review/tests/usage.rs
  - backend/src/review_archive/mod.rs
  - backend/src/review_archive/retention.rs
  - backend/src/server/comment_activation.rs
  - backend/src/server/comment_activation_tests.rs
  - backend/src/server/indexing.rs
  - backend/src/server/mod.rs
  - backend/src/server/polling.rs
  - backend/src/server/polling_pr.rs
  - backend/src/server/publication.rs
  - backend/src/server/queue_acknowledgements.rs
  - backend/src/server/response.rs
  - backend/src/server/review_http.rs
  - backend/src/server/review_jobs.rs
  - backend/src/server/routes.rs
  - backend/src/server/runtime.rs
  - backend/src/server/status_comments.rs
  - backend/src/server/status_comments_test_support.rs
  - backend/src/server/summary.rs
  - backend/src/server/tests.rs
  - backend/src/server/workspace.rs
findings:
  critical: 3
  warning: 0
  info: 2
  total: 5
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-11-05T04:11:56Z
**Depth:** standard
**Files Reviewed:** 30
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

I reviewed the full diff `9d1b8ea..HEAD -- backend/`. The behavior side holds up:

- **Output strings are unchanged.** All five rewritten URL and refspec sites produce the same bytes as the old literals (`commit_url`, `review_url`, `pr_head_ref`).
- **Types line up.** `review_id` is `Option<i64>` or `i64`, `number` is `i64`, and `&String` derefs to `&str` where needed.
- **Every bare `forge::` use resolves.** Each one sits in a module that sees the import through the `use super::*` chain (`server/mod.rs` and `review/mod.rs` import `forge`).
- **Visibility and names are fine.** The glob re-export exposes every item callers use (`pub` / `pub(crate)` at the `github` root, and the `#[cfg(test)]` items under test). No name in `github` collides with `Forge`, `current` or `tests`.
- **No `github::` references remain** outside `github/` and `forge/`.

The problem is formatting. CI runs `cargo fmt --all --check` (`.github/workflows/ci.yml:244`) with `max_width = 120`. rustfmt then uses `fn_call_width = 72`: if a call's arguments are longer than 72 characters on one line, the call is split one argument per line. Three changed sites break this rule, so CI fails before anything else runs.

How I checked the 72-character rule without `cargo`: I scanned the whole backend. Every existing one-line multi-argument call has arguments of 72 characters or fewer. `review/output_repair_tests.rs:175` is exactly 72 and stays on one line. The pre-change `comment_activation_tests.rs:104` was 73 and was split. The only one-line calls over 72 are the new ones listed below. Still, run `cargo fmt --all` in the Docker build image to confirm before pushing.

## Critical Issues

### CR-01: Collapsed `assert_eq!` lines in the forge test fail `cargo fmt --check`

**File:** `backend/src/forge/mod.rs:43-44`
**Issue:** The arguments in these two `assert_eq!` calls are 81 and 92 characters long, which is over the 72 limit. rustfmt splits them one argument per line. Commit `484a69b` ("style(forge): match rustfmt layout in forge test") replaced the correct split layout with these one-line versions, so `cargo fmt --all --check` fails. No other one-line `assert_eq!` in the backend has arguments longer than 72.
**Fix:** Restore the layout from before `484a69b`:
```rust
        assert_eq!(current(), Forge::GitHub);
        assert_eq!(
            Forge::GitHub.commit_url("o/r", "abc123"),
            "https://github.com/o/r/commit/abc123"
        );
        assert_eq!(
            Forge::GitHub.review_url("o/r", 7, 42),
            "https://github.com/o/r/pull/7#pullrequestreview-42"
        );
        assert_eq!(Forge::GitHub.pr_head_ref(7), "refs/pull/7/head");
```

### CR-02: `fetch(...)` in `fetch_summary_refs` now needs one argument per line

**File:** `backend/src/server/summary.rs:249`
**Issue:** The `fetch` arguments are now 84 characters long: `&format!("{}:refs/ocr/summary-{number}", forge::current().pr_head_ref(number)), true`. Before the change they were 67. The last argument, `true`, cannot stay on the first line the way a closure can, so rustfmt splits the call and the `.status()` chain moves down with it. The line as written fails `cargo fmt --check`.
**Fix:**
```rust
    let status = fetch(
        &format!("{}:refs/ocr/summary-{number}", forge::current().pr_head_ref(number)),
        true,
    )
    .status()
    .map_err(|error| error.to_string())?;
```

### CR-03: The rename shortened a split call to exactly 72 characters, so rustfmt now joins it

**File:** `backend/src/server/comment_activation_tests.rs:104-107`
**Issue:** Changing `github::` to `forge::` removed one character. The arguments of the inner `command_number(...)` call went from 73 characters (split) to 72 (fits on one line). This call is an argument in a split `assert_eq!` with nothing after it on its line, so rustfmt now puts it on one line (99 characters with the receiver, well within the 111 available). The split layout left in place fails `cargo fmt --check`. Same 72-character threshold as `review/output_repair_tests.rs:175`.
**Fix:**
```rust
    assert_eq!(
        comment(1).command_number("acme/repo", forge::iso8601_to_epoch_ms("2099-01-01T00:00:00Z").unwrap()),
        None
    );
```
I checked the other renamed lines inside multi-line calls (`find_clean_comment`, `replace_clean_comment`, `retire_clean_comments_before`, the `verify_repository_read_access_using` closure, the `avatar_url` chain, `retention.rs` `let response_bytes =`, `summary.rs` `upsert_issue_comment_by_marker`). Each is still over its width limit after losing the character, so none of them get joined.

## Info

### IN-01: `#[allow(unused_imports)]` on the forge glob re-export does nothing useful

**File:** `backend/src/forge/mod.rs:3`
**Issue:** `unused_imports` only fires on a glob when none of its items are used. This glob is used heavily in both test and non-test builds, so the allow never matters. It would hide the warning if the seam later stopped being used.
**Fix:** Remove the attribute. If clippy complains, keep it with a one-line reason.

### IN-02: The executor fingerprint changes because the rename touched files that are hashed into it

**File:** `backend/src/sandbox/mod.rs:22-46` (hashes `review/mod.rs`, `review/dependencies.rs`, `review/frozen.rs`, `review_archive/mod.rs`, `indexer/mod.rs`, all edited here)
**Issue:** Output strings are unchanged, but `sandbox::engine_version()` now produces a new value. `sandbox/runner.rs:1164` rejects runners with a mismatched version. A server built from this branch next to a runner image from before this phase (for example the upstream `ghcr.io/bridglabs/opencodereview-runner` from `docker-compose.images.yml`) would fail every review. This is expected as long as both images are built from the same commit.
**Fix:** No code change needed. Note in the phase or changelog that the server and runner images must be rebuilt together.

---

_Reviewed: 2026-11-05T04:11:56Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
