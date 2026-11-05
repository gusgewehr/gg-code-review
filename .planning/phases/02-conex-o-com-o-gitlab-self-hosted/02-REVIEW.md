---
phase: 02-conex-o-com-o-gitlab-self-hosted
reviewed: 2026-11-05T00:00:00Z
depth: standard
files_reviewed: 26
files_reviewed_list:
  - .github/workflows/ci.yml
  - backend/src/db/settings.rs
  - backend/src/forge/guard.rs
  - backend/src/forge/mod.rs
  - backend/src/github/auth.rs
  - backend/src/github/mod.rs
  - backend/src/gitlab/auth.rs
  - backend/src/gitlab/auth_tests.rs
  - backend/src/gitlab/ca.rs
  - backend/src/gitlab/mod.rs
  - backend/src/lib.rs
  - backend/src/server/mod.rs
  - backend/src/server/polling.rs
  - backend/src/server/routes.rs
  - backend/src/server/tests.rs
  - changelog.d/feat-gitlab-connection.md
  - docker-compose.yml
  - docker/Dockerfile
  - docker/gitlab-spike.py
  - docker/test-gitlab-isolation.py
  - src/platform/pages/SettingsPage.test.tsx
  - src/platform/pages/SettingsPage.tsx
  - src/platform/pages/settings/GitLabConnection.test.tsx
  - src/platform/pages/settings/GitLabConnection.tsx
  - src/services/setupApiAccess.ts
  - src/types/settings.ts
findings:
  critical: 0
  warning: 6
  info: 3
  total: 9
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-11-05
**Depth:** standard (static only, nothing compiled or run)
**Status:** issues_found

## Summary

The token path is sound. The token goes to glab on stdin only, the glab environment is scrubbed, `redact` runs on all captured output, routes require a trusted origin, and the runner image and compose blocks stay free of GitLab and CA config. The GitHub path is unchanged apart from the host-detection refactor, which matches the stated override. No blockers found. The warnings below are robustness and correctness gaps in the glab wrapper and the UI.

## Warnings

### WR-01: `run_glab` can deadlock on output larger than the pipe buffer

**File:** `backend/src/gitlab/auth.rs:166-186`
**Issue:** stdout and stderr are piped, but they are only read by `wait_with_output` after `try_wait` reports exit. If glab writes more than about 64 KB (verbose errors, a large `api` response), it blocks on write and never exits. The loop then hits the 20 s timeout and reports "glab timed out". `glab api user` is small today, but the wrapper is generic.
**Fix:** Spawn reader threads for stdout and stderr before the wait loop. Alternatively, call `wait_with_output` on a worker thread and enforce the timeout with `recv_timeout`.

### WR-02: A failed login attempt logs out a previously working connection

**File:** `backend/src/gitlab/auth.rs:244-262`
**Issue:** Every failure path in `login_at` (glab error, token problem, missing scope) calls `logout_at`. If an operator is already connected and pastes a bad or expired token, the rollback removes the working credential for that host. The deploy then silently stops being able to review until someone reconnects.
**Fix:** Check whether a login existed before the attempt (`has_stored_login`). Only roll back when there was none. Otherwise restore the old config (copy `config.yml` aside before login), or run glab against a temporary `GLAB_CONFIG_DIR` and move it into place only on success.

### WR-03: The `api` scope check is skipped silently when `personal_access_tokens/self` fails

**File:** `backend/src/gitlab/auth.rs:222-233`
**Issue:** If that second call fails (non-2xx, or an older GitLab or token type that does not support it), the code leaves `logged_in = true` with no scope or expiry validation. A `read_user`-only token is then reported as "Connected" and fails later at review or publish time. `login_at` accepts this status as success.
**Fix:** On a non-success response, either treat it as `Code::Generic` and fail the login, or fall back to a cheap write-capable probe. At minimum, do not report a clean login unless the scope was actually verified.

### WR-04: `forge::host()` falls back to `github.com`, so GitLab URLs are silently wrong before init

**File:** `backend/src/forge/mod.rs:22-24, 120-133`
**Issue:** `host()` returns `"github.com"` when `HOST` is unset. `Forge::GitLab.commit_url` and `review_url` then build `https://github.com/g/p/-/commit/...` with no error. The test `gitlab_urls_and_refspec` asserts exactly that wrong URL, and it passes only because no other test initialises the `OnceLock`. It will break (or depend on test order) as soon as one test calls `init_from_env`. The statics are also process-global and cannot be reset.
**Fix:** Make the GitLab URL builders take the host as a parameter, or return `Option`/`Result` when no host is set. The test then passes an explicit host and does not depend on global state.

### WR-05: `/forge/status` runs up to two blocking glab subprocesses per request with no serialization

**File:** `backend/src/server/routes.rs:183-190`, `backend/src/gitlab/auth.rs:200-236`
**Issue:** Each status call can spawn `glab api user` and `glab api personal_access_tokens/self`, each with a 20 s timeout. This happens on an HTTP connection thread and is re-fetched by the settings page, so a slow or unreachable GitLab can pile up threads. Status, login and logout also run concurrently against the same `config.yml` with no lock, so a status call can race a login or logout.
**Fix:** Guard `login`, `logout` and `status` with one process-wide `Mutex`. Cache the status for a few seconds.

### WR-06: Disconnect errors are never shown, and GitHub recovery UI flashes on GitLab deploys

**File:** `src/platform/pages/settings/GitLabConnection.tsx:14, 70-77`; `src/platform/pages/SettingsPage.tsx:401`
**Issue:**
- `disconnect` is a `useMutation`, but `disconnect.error` is never rendered. If logout fails (the backend returns "Could not disconnect GitLab: ..."), the user sees nothing and the status still reads Connected.
- `authDisconnected && !gitlabForge` is evaluated while `forgeQuery` is still loading, when `gitlabForge` is `null`. On a GitLab deploy, `GitHubAuthRecovery` can therefore flash, or stay visible if `/forge/status` errors.
**Fix:** Render `formatError(disconnect.error)` in the `error` slot. Gate the recovery banner on `forgeQuery.isSuccess && forgeQuery.data.provider === "github"`.

## Info

### IN-01: `has_stored_login` and the fast path in `status_at` are loose heuristics

**File:** `backend/src/gitlab/auth.rs:188-196, 200-205`
**Issue:** `text.contains("token:")` matches anywhere in the file, not under the host key. The early-return condition `!has_stored_login && !config.yml.exists()` is effectively just `!exists()`, and the inner `!has_stored_login` check repeats part of it. The code already flags this as a "shortcut" pending the spike.
**Fix:** Parse the YAML, or drop `has_stored_login` and rely on the exit code of `glab auth status --hostname`.

### IN-02: `classify` matches the bare substring "401"

**File:** `backend/src/gitlab/auth.rs:98-110`
**Issue:** Any glab output containing "401" (a port, an ID, a timestamp) is classified as Unauthorized or Expired. The marker table is also admitted to be unverified.
**Fix:** Match `"401 unauthorized"` or the HTTP status pattern once real glab errors are recorded.

### IN-03: Other GitHub-only background workers still start on a GitLab deploy

**File:** `backend/src/server/mod.rs:288-303`
**Issue:** Only the reindex worker and the poll scheduler are gated on `github`. `start_review_request_restoration`, `start_review_result_reconciliation` and the others still run. I did not trace them, so this is unverified. With no PRs on a GitLab deploy they are probably no-ops, but they could call `gh` against the pinned GitLab host.
**Fix:** Confirm they do nothing without GitHub data, or gate them on `github` too.

---

_Reviewed: 2026-11-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
