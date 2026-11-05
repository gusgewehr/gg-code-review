---
phase: 03-leitura-gitlab-descoberta-e-prepara-o-de-mrs
plan: 04
subsystem: gitlab-discovery
tags: [gitlab, polling, routes, forge]
requires: [03-01, 03-03]
provides: [GitLab polling, GitLab repository refresh, nested workspace routes, glab status via /auth/github/status]
key-files:
  modified: [backend/src/server/polling.rs, backend/src/server/mod.rs, backend/src/server/routes.rs, backend/src/server/response.rs, backend/src/server/publication.rs, backend/src/server/queue_acknowledgements.rs, backend/src/server/tests.rs, backend/src/gitlab/mod.rs, changelog.d/feat-gitlab-connection.md]
decisions:
  - "GitHub-only post-steps (queue acknowledgements, handle_completed_review) return early on GitLab until Phase 4"
metrics:
  tasks: 2
  completed: 2026-11-05
requirements: [REPO-01, REPO-02, REPO-03, REPO-04, REPO-05, FORGE-03]
---

# Phase 3 Plan 04: Enable GitLab discovery Summary

Poll scheduler and `poll_reviews` now run on GitLab (author-command scan stays GitHub-only), refresh lists all projects once and stores one workspace per namespace, nested `g%2Fsub` logins are decoded in routes, and `/auth/github/status` reports glab login so Settings unlocks Repository access.

## Tasks
1. feat(03-04): enable GitLab polling, project refresh and auth status
2. docs(03-04): release note for GitLab merge request discovery

## Deviations from Plan
- Removed the `#[allow(unused_imports)]` for `list_all` (now used); kept it on `valid_full_name`, still unused.

## Verification
fmt + clippy clean; `--lib server::` (86, incl. new nested route test), `gitlab` (25), `forge` (7) pass; changelog check, file-size check and gitlab isolation script pass. `review_archive` (67) passes. Real-instance behavior remains HUMAN-UAT.

## Known Stubs
None.

## Self-Check: PASSED
