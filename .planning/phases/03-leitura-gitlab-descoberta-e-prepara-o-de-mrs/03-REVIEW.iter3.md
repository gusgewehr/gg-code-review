---
phase: 03-leitura-gitlab-descoberta-e-prepara-o-de-mrs
reviewed: 2026-11-05T14:32:58Z
depth: standard
iteration: 3
files_reviewed: 24
files_reviewed_list:
  - backend/src/db/repositories.rs
  - backend/src/forge/mod.rs
  - backend/src/github/auth.rs
  - backend/src/github/mod.rs
  - backend/src/github/transport.rs
  - backend/src/gitlab/api.rs
  - backend/src/gitlab/auth.rs
  - backend/src/gitlab/git_auth.rs
  - backend/src/gitlab/mod.rs
  - backend/src/gitlab/mrs.rs
  - backend/src/gitlab/projects.rs
  - backend/src/gitlab/read_tests.rs
  - backend/src/review/frozen.rs
  - backend/src/review_archive/mod.rs
  - backend/src/review_archive/retention.rs
  - backend/src/server/mod.rs
  - backend/src/server/polling.rs
  - backend/src/server/publication.rs
  - backend/src/server/queue_acknowledgements.rs
  - backend/src/server/response.rs
  - backend/src/server/routes.rs
  - src/platform/lib/utils.ts
  - src/platform/pages/SettingsPage.tsx
  - src/platform/pages/settings/RepositorySelector.tsx
findings:
  critical: 1
  warning: 1
  info: 9
  total: 11
status: issues_found
---

# Phase 03: Code Review Report (iteration 3)

**Reviewed:** 2026-11-05T14:32:58Z
**Depth:** standard
**Files Reviewed:** 24
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

This pass re-checks the fix commits `a9a01a2..a17224f` against the iteration-2 review, and looks for regressions they introduce.

| Prior finding | Verdict |
|---|---|
| CR-01 nested routes behind nginx | **Fixed.** `["workspaces", login @ .., "repos"]` and the `refresh` arm are correct. The trailing literal stops them from capturing other routes. A subgroup literally named `repos` still resolves correctly, because the slice pattern binds the last segment. The new test covers the decoded path. |
| CR-02 NULL base SHA from polling | **Fixed, but the fix adds a new blocker** (see CR-01 below). The happy path is correct: rows written earlier with NULL give `pr_scope = None`, so the first poll after upgrade does not mass-reset MRs. |
| WR-01 stuck in `reviewing` | **Fixed.** It is guarded by the caller's `finished` check, so a superseded run cannot move the lane. See IN-01 for a side effect. |
| WR-02 cancellation and lock scope | **Mostly fixed.** Cancellation kills glab, and the read lock is now taken per attempt. Nothing still holds the outer guard, so the read lock is never taken twice. The 120 s hard cap was not addressed (WR-01 below). |
| WR-03 unlocked config read | **Fixed.** The guard is dropped before git runs. No caller of `forge::apply_git_https_auth` holds the glab lock. |

**GitHub flow:** no regressions found. Every new gate is `Forge::GitLab`-only. `basic_auth_token` gives the same bytes as before, and the upsert SQL is unchanged.

**Token exposure:** none found. glab still gets the token only on stdin. git gets it only through child env. The new `branch_tip_at` errors are the fixed generic string, and the logs carry only the class code.

## Critical Issues

### CR-01: One MR whose target branch cannot be resolved stops all polling for its project

**File:** `backend/src/gitlab/mrs.rs:174-182` (consumer: `backend/src/server/polling.rs:141-203`)
**Issue:** `list_open_at` uses `?` to propagate any `branch_tip_at` failure, so the whole listing fails. GitLab keeps an MR open when its target branch is deleted outside the merge-and-retarget flow (deleted in the UI, `git push --delete`, or an expired release branch). The MR widget then shows "target branch does not exist". The `GET repository/branches/<name>` call returns 404 every time, so the failure is persistent, not transient. In `run_poll_reviews`, `Err(error)` skips the project entirely. While that one MR stays open:
- New MRs in the project are never discovered, and new heads are never reviewed. This breaks the core value.
- `reconcile_pr_lifecycle` never runs for the project. Merged and closed MRs are never marked terminal, and running reviews of closed MRs are not superseded. `wake_retention_worker` is never triggered, and GitLab has no other merge backfill (`retention.rs:381` returns early on GitLab), so archived source code of merged MRs is kept past the retention policy.
- `eligible_queue` misses the project's PRs, so queued comment-activated reviews are not started either.
- The only signal is a poll error, and it is misleading: `branch_tip_at` passes `what = "merge requests"`, so it reads "Could not load merge requests from <host>. Check the connection and refresh repository access." Nothing points at the branch or the MR. Refreshing access does not help.

This is a blocker. One user action that GitLab allows silently disables review and lifecycle tracking for every MR in the project, with no hint at the cause. The fix report flagged the risk and assumed "GitLab normally retargets or closes those MRs". That assumption holds only when the branch is removed by merging its own MR with "delete source branch".

The new test `open_list_fills_base_tip_once_per_branch` covers only the success path.

**Fix:** Degrade per branch, and stop writing NULL over a known base:
```rust
// mrs.rs, list_open_at
let mut tips: HashMap<String, Option<String>> = HashMap::new();
for pr in &mut prs {
    let branch = pr.base_ref_name.clone().unwrap_or_default();
    if !tips.contains_key(&branch) {
        let tip = match branch_tip_at(bin, host, dir, full_name, &branch, backoff_ms, &no) {
            Ok(tip) => Some(tip),
            Err(e) if e.starts_with("GitLab rejected") => return Err(e), // token problems still fail the list
            Err(_) => None, // deleted/unreadable target: keep listing the project
        };
        tips.insert(branch.clone(), tip);
    }
    pr.base_ref_oid = tips.get(&branch).cloned().flatten();
}
```
```sql
-- db/repositories.rs:81 (GitHub always sends baseRefOid, so its behavior does not change)
base_sha = COALESCE(excluded.base_sha, pull_requests.base_sha),
```
A `None` base only turns off poll-time base-change detection for that one MR. Preparation still fails safely in `live_state_at`, because it calls `branch_tip_at` again. Add a test where one branch lookup exits 1: the other MRs must still be listed, with their tips. Also pass `what = "branches"`, or a similar name, to `branch_tip_at` so its error names the right thing.

## Warnings

### WR-01: The 120 s hard cap on `glab api --paginate` is still in place (WR-02 bullet 3, not addressed)

**File:** `backend/src/gitlab/api.rs:67` (callers: `projects.rs:84`, `mrs.rs:165`)
**Issue:** Each attempt gets `Duration::from_secs(120)`. On an instance with thousands of reachable projects, a full paginated `projects` listing that takes more than 120 s always times out. The retry then starts again from page 1 and hits the same timeout, so Settings can never list repositories. `run_glab_cancellable` maps the timeout to the same generic error, so the user cannot tell it apart from a connection problem. The backoff `thread::sleep` (up to 7 s) also ignores `cancel`. That part is minor, because `cancel` is checked again right after the sleep.
**Fix:** Use a larger per-call budget for paginated calls, for example `let timeout = if paginate { 600 } else { 120 };`, and skip retrying a timeout on a paginated listing. Or filter the listing to cut its size (`membership=true` / `min_access_level=30`, if the project listing does not already do this).

## Info

### IN-01: The WR-01 fix creates a "ready for your approval" pending action that cannot be acted on

**File:** `backend/src/server/publication.rs:558-559` (`db/review_runs.rs:530-541`)
**Issue:** `mark_review_ready_for_run` calls `upsert_pending_action_for_run("review_ready", …, "A generated review is ready for your approval.")`. On GitLab, approving reaches gh and fails at `require_github`. Clean and findings runs both land in `ready` with no `review_result` row, so a clean MR looks the same as one with findings. This is an acceptable interim state for Phase 3. Phase 4 must replace it.
**Fix:** Optional: call `set_pr_lane` alone, without creating the pending action, until Phase 4.

### IN-02: Preparation-time `pr_description` and polling list calls are not cancellable

**File:** `backend/src/gitlab/mrs.rs:228-238`, `:172`
**Issue:** These calls use a fresh `AtomicBool::new(false)`. A cancelled review can still wait out one `pr_description` retry cycle. The GitHub version has the same behavior (`run_raw`), so this is not a regression.

### IN-03 – IN-09: Carried over from iteration 2 (not in fix scope, still present)

- **IN-03:** `routes.rs:179` hardcodes `"detail": "glab authenticated"` and drops `ForgeStatus.error`.
- **IN-04:** `polling.rs:254-256` uses the reasons "PR merged on GitHub" / "PR closed on GitHub" for GitLab MRs.
- **IN-05:** The assertion at `read_tests.rs:303-306` is tautological (it compares `basic_auth_for` with itself). The `glpat-` check in `exhausts_on_502_without_leaking` inspects argv instead of the returned error.
- **IN-06:** `mrs.rs:67` sets `changed_files: Some(0)`, so the poll-time size limit never applies on GitLab.
- **IN-07:** `review_archive/mod.rs:270` (`names.len() < 2`) also loosens the repository-name contract on GitHub deploys.
- **IN-08:** `api.rs:36-40`: `retryable` matches bare status-code tokens anywhere in stderr.
- **IN-09:** `branch_tip_at` (`mrs.rs:115`) reports its failures as "merge requests". See the CR-01 fix.

---

_Reviewed: 2026-11-05T14:32:58Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
