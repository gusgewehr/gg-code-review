---
phase: 04-escrita-gitlab-publica-o-dos-resultados
plan: 01
subsystem: gitlab
tags: [gitlab, glab, sanitizer, diff-positions]
requires: []
provides:
  - "gitlab::sanitize::finalize_body / strip_zwsp"
  - "gitlab::write::api_write_at / write_json / WriteError"
  - "gitlab::diffs::parse_diffs / position_for / anchors_from / diffs_at / diff_anchors"
  - "gitlab::test_fake shared fake glab (unix tests)"
affects: [04-02, 04-03, 04-04]
tech-stack:
  added: []
  patterns: ["glab api --input <0600 temp file>, never -f/-F", "POST never retried on 5xx/timeout"]
key-files:
  created:
    - backend/src/gitlab/sanitize.rs
    - backend/src/gitlab/write.rs
    - backend/src/gitlab/diffs.rs
    - backend/src/gitlab/test_fake.rs
  modified:
    - backend/src/gitlab/mod.rs
    - backend/src/github/mod.rs
    - backend/src/github/comments.rs
key-decisions:
  - "Hand-rolled line scanner for sanitizer, no regex crate"
  - "WriteError splits Rejected/NotFound/Other so the publisher can fall back to a general note"
requirements-completed: [PUB-01, PUB-02, PUB-07]
duration: 40min
completed: 2026-11-05
---

# Phase 4 Plan 01: GitLab write primitives Summary

Body sanitizer (quick actions, mentions, suggestion fences, 1M cap), JSON write helper over `glab api --input` with no POST retry on 5xx, and an MR diff line map with position builder, all unit-tested.

## Deviations from Plan

- Single commit for the three tasks instead of one each: all three share `gitlab/mod.rs` registrations, and splitting them would have produced non-building intermediate commits. Commit 818f302.
- Test fixture for `diffs_at` avoids `\n` in the diff string because `sh` echo expands it and breaks the JSON.

## Verification

`--lib gitlab` 54 passed, `--lib github` 36 passed, fmt and clippy `-D warnings` clean.

## Known Stubs

None. New modules carry `#[allow(dead_code)]` until the publisher (04-03) consumes them; removal planned in 04-04.

## Self-Check: PASSED
