---
phase: 03-leitura-gitlab-descoberta-e-prepara-o-de-mrs
plan: 03
subsystem: gitlab-review-prep
tags: [gitlab, git-auth, scope, forge]
requires: [03-01]
provides: [gitlab::apply_git_auth, forge::apply_git_https_auth, nested Scope identity]
key-files:
  created: [backend/src/gitlab/git_auth.rs]
  modified: [backend/src/github/auth.rs, backend/src/github/transport.rs, backend/src/forge/mod.rs, backend/src/review/frozen.rs, backend/src/review_archive/mod.rs, backend/src/review_archive/retention.rs, backend/src/db/repositories.rs]
decisions:
  - "GitLab git token read from glab config.yml per call, sent as oauth2 extraheader via GIT_CONFIG_* env only"
  - "gh calls fail closed on GitLab in transport.rs (one place)"
metrics:
  tasks: 2
  completed: 2026-11-05
requirements: [REV-01, REV-02, REV-03, FORGE-03]
---

# Phase 3 Plan 03: GitLab review preparation Summary

Env-only oauth2 extraheader git auth for the GitLab host, fail-closed `gh` guard, dependency-capture and merge-backfill skips on GitLab, and nested `g/sub/proj` accepted by `Scope::validate` and `rsplit_once` in the DB helper.

## Tasks
1. Git transport auth: feat(03-03) commit "GitLab git transport auth via env extraheader"
2. gh guard, skips, nested identity: feat(03-03) commit "fail-closed gh guard, GitLab prep skips, nested repository identity"

## Deviations from Plan
None of substance. The "basic_auth_token unchanged" check is covered by the existing github tests plus a trivially-weak assertion in the new test.

## Verification
fmt + clippy clean; `--lib gitlab` (25), `github` (36), `review_archive` (67), `review::` (286), `db::` (106) pass; file-size check passes. config.yml token layout and real fork-MR fetch with internal CA remain HUMAN-UAT. `docker/test-gitlab-isolation.py` not run (no runner change).

## Known Stubs
None.

## Self-Check: PASSED
