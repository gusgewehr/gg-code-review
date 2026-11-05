---
phase: 04-escrita-gitlab-publica-o-dos-resultados
plan: 02
subsystem: server
tags: [gitlab, status-comments, review-policy]
requires: []
provides:
  - "StatusCommentApi::edit (default Ok(false)) and delete(repo, number, id)"
  - "review::findings::post_event_mode, forge-aware auto_post_event"
  - "GitLab decision guard in decide_review_http; GitLab /-/commit/ link in queued text"
affects: [04-03, 04-04]
key-files:
  modified:
    - backend/src/github/status_comments.rs
    - backend/src/server/status_comments.rs
    - backend/src/server/status_comments_test_support.rs
    - backend/src/server/status_comments_tests.rs
    - backend/src/review/findings.rs
    - backend/src/server/review_http.rs
    - backend/src/server/comment_activation.rs
key-decisions:
  - "Edit hook lives in replace_owned_comment: oldest replaceable owned note is edited, edit Err aborts before any delete"
  - "Coercion of request_changes to comment happens inside auto_post_event so publication and the db auto-confirm guard agree"
requirements-completed: [PUB-03, PUB-06]
completed: 2026-11-05
---

# Phase 4 Plan 02: Status-note edit hook and GitLab review policy Summary

Status comments can now be edited in place when the forge supports it (GitHub default stays post-then-delete), and GitLab can never send approve or request-changes: auto mode coerces to COMMENT, manual decisions are rejected, and queued text links `/-/commit/`.

## Commits
- 7702822 Task 1: edit hook, delete signature, fake extension, 3 new tests
- 862e84f Task 2: findings/review_http/comment_activation helpers and tests

## Deviations from Plan
None. Nothing in `db/review_runs.rs` needed editing: it already calls `auto_post_event`.

## Verification
`--lib server::status_comments` 16 passed, `--lib review::findings` 9, `--lib server::` 91, `--lib github` 36; fmt and clippy `-D warnings` clean. Existing status_comments assertions untouched.

## Known Stubs
None.

## Notes
STATE.md updates skipped per instructions where gsd-sdk cannot parse it.

## Self-Check: PASSED
