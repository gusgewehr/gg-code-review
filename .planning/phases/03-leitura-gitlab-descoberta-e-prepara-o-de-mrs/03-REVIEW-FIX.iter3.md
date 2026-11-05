---
phase: 03-leitura-gitlab-descoberta-e-prepara-o-de-mrs
fixed_at: 2026-11-05T00:00:00Z
review_path: .planning/phases/03-leitura-gitlab-descoberta-e-prepara-o-de-mrs/03-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 03: Code Review Fix Report

**Fixed at:** 2026-11-05
**Source review:** .planning/phases/03-leitura-gitlab-descoberta-e-prepara-o-de-mrs/03-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5
- Fixed: 5
- Skipped: 0

Verification: `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings` are clean. `cargo test --lib -- gitlab server:: forge::` passed (117 tests).

## Fixed Issues

### CR-01: Nested-group workspace routes break behind nginx

**Files modified:** `backend/src/server/routes.rs`, `backend/src/server/tests.rs`
**Commit:** 04a6ae2
**Applied fix:** The workspace routes now match `["workspaces", login @ .., "repos"]` and the `refresh` variant (the refresh arm comes first, and a guard rejects an empty login). The segments are joined with `/` and then percent-decoded. A single-segment GitHub login gives the same string as before. The existing test now also sends the decoded path `/workspaces/g/sub/repos`, which is what nginx forwards.

### CR-02: GitLab polling writes NULL over `pull_requests.base_sha`

**Files modified:** `backend/src/gitlab/mrs.rs`, `backend/src/gitlab/read_tests.rs`
**Commit:** a9a01a2
**Applied fix:** I moved the branch-tip lookup out of `live_state_at` into a shared `branch_tip_at`. A new `list_open_at` lists the MRs, then looks up each distinct `target_branch` tip once per call and puts it in `base_ref_oid`. Polling now gets a real base, so `base_scope_changed` and `db.pr_scope` work for GitLab the same way they do for GitHub. If a tip lookup fails, the whole list fails. That matches a failed GitHub list and never writes NULL. The upsert SQL is unchanged, so GitHub behavior is untouched. New test: `open_list_fills_base_tip_once_per_branch`.
**Status:** fixed: requires human verification (logic change: when a target branch is deleted, polling now fails for every MR in the project. GitLab normally retargets or closes those MRs.)

### WR-01: A completed GitLab review leaves the PR stuck in the `reviewing` lane

**Files modified:** `backend/src/server/publication.rs`
**Commit:** 68984e6
**Applied fix:** The GitLab early return in `handle_completed_review` now calls `mark_review_ready_for_run` and then `set_pr_lane(.., "ready", "Publishing to GitLab arrives in Phase 4")`. It still returns before every step that reaches gh. Clean and findings results both land in `ready` for now. The full result bookkeeping arrives with Phase 4 publishing.
**Status:** fixed: requires human verification (state transition)

### WR-02: GitLab API calls ignore cancellation and can hold the glab read lock for minutes

**Files modified:** `backend/src/gitlab/api.rs`, `backend/src/gitlab/auth.rs`, `backend/src/gitlab/mrs.rs`, `backend/src/gitlab/read_tests.rs`
**Commit:** a9a6812
**Applied fix:**
- Added `run_glab_cancellable`, which kills the child and returns `"cancelled"` as soon as cancel is set. `run_glab` delegates to it with a false flag, so the auth callers are unchanged.
- `api_at` takes `cancel`, checks it before each attempt (including after a backoff sleep), and passes through `"cancelled"` instead of the generic error.
- `glab_read()` is now taken per attempt inside `api_at`. The outer guards in `api`, `list_open_prs`, `live_pr_state_cancellable` and `pr_description` are gone, so the read lock is never taken twice.
- `live_state_at` passes the review's cancel flag through every call. I added `#[allow(clippy::too_many_arguments)]` on `api_at`; the codebase already does this in 4 other places.
- New test: `cancel_kills_a_running_call`.
- Not addressed: the third bullet, the 120 s cap on a long paginated `projects` listing. The Fix section prescribed nothing for it.

### WR-03: `apply_git_auth` reads glab `config.yml` without the glab lock

**Files modified:** `backend/src/gitlab/git_auth.rs`
**Commit:** a17224f
**Applied fix:** The file is now read inside a block that holds `super::auth::glab_read()`. The guard is dropped before git runs. Its only caller, `forge::apply_git_auth`, holds no glab lock, so the read lock is not taken twice.

---

_Fixed: 2026-11-05_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
