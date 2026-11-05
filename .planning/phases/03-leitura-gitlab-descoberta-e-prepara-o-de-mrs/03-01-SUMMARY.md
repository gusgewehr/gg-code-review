---
phase: 03-leitura-gitlab-descoberta-e-prepara-o-de-mrs
plan: 01
subsystem: gitlab-read
tags: [gitlab, glab, forge, mr, projects]
requires: [phase-02 glab auth]
provides: [gitlab::api, gitlab::projects, gitlab::mrs, forge read dispatch]
affects: [server/polling, review/execution, server/publication]
key-files:
  created: [backend/src/gitlab/api.rs, backend/src/gitlab/projects.rs, backend/src/gitlab/mrs.rs, backend/src/gitlab/read_tests.rs]
  modified: [backend/src/gitlab/auth.rs, backend/src/gitlab/mod.rs, backend/src/forge/mod.rs]
decisions:
  - "Dropped simple=true from the project listing so visibility is available for Private/Public"
  - "GLAB_LOCK is an RwLock: login/logout write, status and API reads share"
  - "activation_pr returns Ok(None) on GitLab; gh_host returns the pinned GitLab host"
metrics:
  tasks: 2
  completed: 2026-11-05
requirements: [REPO-01, REPO-02, REPO-03, REPO-04, REPO-05, REV-01]
---

# Phase 3 Plan 01: GitLab read layer Summary

`glab api` runner (120s timeout, 3 bounded 429/5xx retries, tolerant paginator, percent-encoder), project and MR mapping onto the GitHub types, and `forge::` dispatch wrappers that shadow the github glob so no caller changed.

## Tasks
1. glab api runner, paginator, encoder: 07d97aa
2. Projects/MRs mapping and forge wrappers: bb84933

## Deviations from Plan
- [Rule 1] `list_all` and `valid_full_name` are re-exported from `gitlab/mod.rs` under `#[allow(unused_imports)]` because clippy `-D warnings` rejects them until later plans consume them.
- Task 1 and 2 commits split by file, so the first commit alone does not compile (api.rs depends on mod.rs wiring committed in the second).
- Plan-specified `simple=true` removal is as planned (documented in code).

## Verification
fmt + clippy clean; `--lib gitlab` (23 tests) and `--lib forge` (7) pass; file-size check passes. Real glab output shape, `draft`/`reviewers` presence and 429 wording remain HUMAN-UAT.

## Known Stubs
`changed_files: Some(0)` on MRs is intentional (list has no file count).

## Self-Check: PASSED
