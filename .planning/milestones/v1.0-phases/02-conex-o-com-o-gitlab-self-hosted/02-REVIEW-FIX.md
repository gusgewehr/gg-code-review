---
phase: 02-conex-o-com-o-gitlab-self-hosted
fixed_at: 2026-11-05T00:00:00Z
review_path: .planning/phases/02-conex-o-com-o-gitlab-self-hosted/02-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 5
skipped: 1
status: partial
---

# Phase 02: Code Review Fix Report

**Source review:** 02-REVIEW.md (scope: critical + warning)
**Iteration:** 1

Worked directly on branch docs/codebase-map (no worktree, per orchestrator rule not to switch branches).

## Fixed Issues

### WR-01: run_glab pipe deadlock
**Files:** `backend/src/gitlab/auth.rs` | **Commit:** aaf8a0b
stdout/stderr are drained by reader threads while the wait loop runs.

### WR-02: failed login logs out working connection
**Files:** `backend/src/gitlab/auth.rs` | **Commit:** aaf8a0b (requires human verification)
If a stored login existed before the attempt, a glab-rejected login no longer calls logout. Limit: if glab accepts the token but the post-check (scope/expiry) fails, the new token has already replaced the old one, so rollback logout is still performed.

### WR-05: concurrent glab calls on config.yml
**Files:** `backend/src/gitlab/auth.rs` | **Commit:** aaf8a0b
Process-wide mutex around login, status and logout. The short status cache was not added.

### WR-04: github.com fallback in GitLab URLs
**Files:** `backend/src/forge/mod.rs` | **Commit:** a3fe0f9
URL builders now take the host explicitly (`gitlab_commit_url`, `gitlab_review_url`); the test passes a host and no longer depends on global state.

### WR-06: disconnect errors hidden, GitHub recovery flash
**Files:** `src/platform/pages/settings/GitLabConnection.tsx`, `src/platform/pages/SettingsPage.tsx` | **Commit:** 166828d
Disconnect error is rendered; recovery banner waits until the forge status query has settled.

## Skipped Issues

### WR-03: api scope check skipped when personal_access_tokens/self fails
**File:** `backend/src/gitlab/auth.rs:222-233`
**Reason:** Failing or probing differently depends on how real GitLab/glab respond for unsupported token types (spike data not available); a fix now would be speculative.

## Verification
cargo fmt/clippy `-D warnings` clean; `--lib gitlab` (12) and `--lib forge` (7) pass; vitest (22), tsc and biome lint clean. `server::tests` not run (no server files changed).

---
_Fixer: Claude (gsd-code-fixer)_
