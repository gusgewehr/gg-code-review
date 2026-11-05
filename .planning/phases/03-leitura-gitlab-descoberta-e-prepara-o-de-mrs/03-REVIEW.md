---
phase: 03-leitura-gitlab-descoberta-e-prepara-o-de-mrs
reviewed: 2026-11-05T14:10:55Z
depth: standard
files_reviewed: 29
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
  - backend/src/review_archive/tests.rs
  - backend/src/server/mod.rs
  - backend/src/server/polling.rs
  - backend/src/server/publication.rs
  - backend/src/server/queue_acknowledgements.rs
  - backend/src/server/response.rs
  - backend/src/server/routes.rs
  - backend/src/server/tests.rs
  - src/pages/OnboardingRepos.tsx
  - src/platform/lib/utils.ts
  - src/platform/lib/utils.test.ts
  - src/platform/pages/SettingsPage.tsx
  - src/platform/pages/settings/RepositorySelector.tsx
  - src/platform/pages/settings/RepositorySelector.test.tsx
findings:
  critical: 2
  warning: 3
  info: 6
  total: 11
status: issues_found
---

# Phase 03: Code Review Report

**Reviewed:** 2026-11-05T14:10:55Z
**Depth:** standard
**Files Reviewed:** 29
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

I reviewed the diff `97a0f51..HEAD`: the GitLab read layer (glab api runner, paginator, project and MR mapping, git transport auth), forge dispatch, nested-namespace identity, the polling and publication gates, and the forge-aware Settings copy. I found no regressions in the GitHub flow. Every `forge::` dispatch keeps the old GitHub call, `rsplit_once` and `lastIndexOf` give the same result for `owner/name`, and `percent_decode` does not change GitHub logins.

I found no GitLab token exposure. The token goes to glab only on stdin. git gets it only through child env. Errors are fixed strings, and logs carry only the class code. Nested paths cannot escape the repository directory. `parse_projects` validates `path_with_namespace` and requires it to equal `namespace.full_path/path`, so `owner` and `name` (used in `repos_dir.join(owner).join(name)`) are validated too. Both JSON shapes `glab api --paginate` can print (one array, or several arrays back to back) are handled.

Two defects block shipping:
1. Nested-group routes break behind the production nginx.
2. GitLab polling writes NULL over the PR's base SHA, which disables detection of base-branch changes.

## Critical Issues

### CR-01: Nested-group workspace routes break behind nginx (`%2F` is decoded before it reaches the server)

**File:** `backend/src/server/routes.rs:81-82` (with `docker/web-nginx.conf:17-18`)
**Issue:** The frontend calls `/api/workspaces/${encodeURIComponent("g/sub")}/repos`, so the URL holds `g%2Fsub`. In `docker/web-nginx.conf`, `proxy_pass http://server:8787/;` includes a URI part. With a URI part, nginx forwards the *normalized* URI: it decodes `%2F` to `/` and does not re-encode `/` (`NGX_ESCAPE_URI`). The server therefore receives `/workspaces/g/sub/repos`. That splits into `["workspaces","g","sub","repos"]`, which matches neither 3-segment pattern, and the request fails.

The new test `nested_workspace_login_is_percent_decoded_in_routes` calls `route_dynamic` directly with the raw `%2F` path, so it passes. The Vite dev proxy also keeps the raw path, so `pnpm debug:web` works. In the Docker Compose runtime, listing and refreshing projects of any subgroup fails, and subgroups are the core GitLab case.
**Fix:** Accept multi-segment logins in the router. Keep the decode for the dev proxy:
```rust
("GET", ["workspaces", login @ .., "repos"]) if !login.is_empty() => {
    list_workspace_repos(state, &percent_decode(&login.join("/")))
}
("POST", ["workspaces", login @ .., "repos", "refresh"]) if !login.is_empty() => {
    refresh_workspace_repos(state, &percent_decode(&login.join("/")))
}
```
Put the `refresh` arm before the `repos` arm. Add a test that sends the decoded form `/workspaces/g/sub/repos`. Do not rely on an nginx `$request_uri` rewrite.

### CR-02: GitLab polling writes NULL over `pull_requests.base_sha`, so base changes and retargets are never detected

**File:** `backend/src/gitlab/mrs.rs:61` (consumers: `backend/src/server/polling_pr.rs:151-170`, `backend/src/db/repositories.rs:81`, `backend/src/server/review_jobs.rs:150-158`)
**Issue:** `to_pull_request` sets `base_ref_oid: None`. Three things follow on every GitLab poll:
- `upsert_pull_request_with_github_updated_at` runs `base_sha = excluded.base_sha` and sets the column to NULL. This erases the live base that `refresh_review_base` saved at review start.
- In `poll_one_pr`, `incoming_scope` is `None` (`base_ref_name.zip(base_ref_oid)`), so `base_scope_changed` is always false.
- `db.pr_scope` filters on `base_sha IS NOT NULL`, so it returns `None`. The review-start check in `review_jobs.rs` (`invalidate_review_scope("PR base changed…")`) therefore never runs either.

As a result, a GitLab MR that is retargeted (for example from `main` to `release`), or whose target branch moves in a way that changes the merge-base, keeps its decided or reviewed state. It is never reset or re-reviewed until the head SHA changes. GitHub handles this case through `poll_scope_needs_reset(base_scope_changed)`. This breaks the "scope with live refs" invariant.
**Fix:** Supply a real base SHA when listing. The MR list endpoint has no `diff_refs`, so resolve each distinct `target_branch` once per project per poll with the existing branches call, then fill `base_ref_oid`. Example: add `list_open_prs` post-processing.
```rust
let mut tips: HashMap<String, Option<String>> = HashMap::new();
for pr in &mut prs {
    let branch = pr.base_ref_name.clone().unwrap_or_default();
    let tip = tips.entry(branch.clone()).or_insert_with(|| branch_tip(full_name, &branch).ok());
    pr.base_ref_oid = tip.clone();
}
```
At minimum, the upsert must not write NULL over a known base (`base_sha = COALESCE(excluded.base_sha, pull_requests.base_sha)`; GitHub always sends a value, so its behavior does not change). That alone still does not detect base moves at poll time.

## Warnings

### WR-01: A completed GitLab review leaves the PR stuck in the `reviewing` lane

**File:** `backend/src/server/publication.rs:555-558`
**Issue:** The GitLab early return in `handle_completed_review` comes before every lane and result transition. That includes the steps that never call gh: `mark_review_ready_for_run`, `set_pr_lane(.., "ready", ..)`, and the local `clean`/`findings` result bookkeeping. `start_review_skeleton` sets the lane to `reviewing` with "AI review queued" (`db/review_runs.rs:79`), and `finish_review_run` only moves the run to `preview`. After a successful GitLab review the PR therefore stays in `reviewing` / "AI review queued" indefinitely. The board never shows it as ready or clean, and the finding selection UI is unreachable. `poll_one_pr` counts `reviewing` as a managed lane, so polling does not fix it either.
**Fix:** Gate only the gh-reaching calls, not the whole function. At minimum, set the local state before returning on GitLab:
```rust
if crate::forge::current() == crate::forge::Forge::GitLab {
    log_auto_post(inputs, "not attempted: publishing to GitLab arrives in Phase 4");
    let _ = db.mark_review_ready_for_run(inputs.pr_id, inputs.run_id);
    let _ = db.set_pr_lane(inputs.pr_id, "ready", Some("Publishing to GitLab arrives in Phase 4"));
    return;
}
```

### WR-02: GitLab API calls ignore cancellation and can hold the glab read lock for minutes

**File:** `backend/src/gitlab/api.rs:57-74`, `backend/src/gitlab/mrs.rs:161-175`
**Issue:**
- `live_pr_state_cancellable` checks `cancel` only between calls. Inside one call, `api_at` runs `run_glab` with a 120 s timeout, up to 4 attempts, plus 1 s/2 s/4 s backoff. A cancelled or superseded review can sit in preparation for about 8 minutes. The GitHub version kills `gh` as soon as cancel is set (`gh_capture_cancellable`).
- The whole sequence runs under `glab_read()`, so `login` and `logout` (`glab_write()`) block an HTTP request for the same time.
- A paginated `projects` listing on a large instance that takes longer than 120 s always fails with the generic error.
**Fix:** Pass `cancel` into `api_at` and `run_glab`, and kill the child when it is set (the poll loop already exists). Check `cancel` before each retry sleep. Take the read lock per attempt, not around the full retry loop.

### WR-03: `apply_git_auth` reads glab `config.yml` without the glab lock

**File:** `backend/src/gitlab/git_auth.rs:51-53`
**Issue:** Every other read or write of the shared `config.yml` goes through `GLAB_LOCK`. This one reads it directly. If a Reconnect (`glab auth login`, holding the write lock) rewrites the file at the same moment, the read can see a truncated file. The token is then missing, git runs unauthenticated, the fetch fails, and preparation fails with "could not fetch PR branch". The failure is safe but random, and a Reconnect is exactly when a new token is being saved.
**Fix:** `let text = { let _guard = super::auth::glab_read(); std::fs::read_to_string(...) }` (drop the guard before git runs).

## Info

### IN-01: The GitLab auth status hardcodes `detail` and drops the error

**File:** `backend/src/server/routes.rs:171-176`
**Issue:** `"detail": "glab authenticated"` is returned even when `loggedIn` is false. `ForgeStatus.error` (expired token, missing `api` scope, TLS or unreachable host) is discarded, so Settings shows "GitLab is not connected" with no reason.
**Fix:** Forward `v["error"]` and derive `detail` from `loggedIn`.

### IN-02: Lifecycle reasons say "GitHub" for GitLab MRs

**File:** `backend/src/server/polling.rs` (`reconcile_pr_lifecycle`, "PR merged on GitHub" / "PR closed on GitHub")
**Issue:** GitLab MRs are now reconciled through this path, so users see the GitHub wording.
**Fix:** Use forge-neutral text, or switch on `forge::current()`.

### IN-03: A test assertion is tautological, and a "no leak" check inspects the wrong stream

**File:** `backend/src/gitlab/read_tests.rs:250-253`, `:155`
**Issue:** `basic_auth_for("x-access-token","t") == basic_auth_for("x-access-token","t")` compares the function with itself, so it does not guard the GitHub `basic_auth_token` refactor. In `exhausts_on_502_without_leaking`, the token is printed to stderr, but the test checks `args.log` (argv), which can never contain it. The `assert_eq!(err, generic)` is the only real check.
**Fix:** Assert `basic_auth_token("t") == "eC1hY2Nlc3MtdG9rZW46dA=="`, and delete the args.log `glpat-` assertion or point it at the returned error.

### IN-04: `changed_files: Some(0)` means the poll-time size limit never applies on GitLab

**File:** `backend/src/gitlab/mrs.rs:66`
**Issue:** `too_many_files` is always false, so every oversized MR starts a review (clone, fetch, run record) before `execution.rs:187` skips it. The cost and noise are bounded, and this is a documented shortcut.
**Fix:** Optional: use `changes_count` from the single-MR endpoint when the count is needed.

### IN-05: The archive scope accepts 3+ segment repositories on GitHub deploys too

**File:** `backend/src/review_archive/mod.rs:270`
**Issue:** `names.len() < 2` loosens the contract for every forge. There is no traversal risk because segments are still validated. A malformed GitHub `owner/name/extra` is just no longer rejected.
**Fix:** `|| (crate::forge::current() == Forge::GitHub && names.len() != 2)`.

### IN-06: `retryable` matches status codes anywhere in stderr

**File:** `backend/src/gitlab/api.rs:35-39`
**Issue:** Any stderr token equal to `500`, `502`, etc. triggers a retry, for example a project or MR id in an error message, which adds up to about 7 s of backoff to a non-transient failure.
**Fix:** Match glab's `HTTP <code>` / `<code> <reason>` prefix instead of bare tokens.

---

_Reviewed: 2026-11-05T14:10:55Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
