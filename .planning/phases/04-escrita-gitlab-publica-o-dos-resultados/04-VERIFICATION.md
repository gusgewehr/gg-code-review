---
phase: 04-escrita-gitlab-publica-o-dos-resultados
verified: 2026-11-05T00:00:00Z
status: human_needed
score: 5/5 must-haves verified (automated, fake-glab); real-GitLab wire behavior pending
human_verification:
  - test: "Real /diffs and /versions shapes; inline discussion on added, removed and context lines"
    expected: "Discussions land on the correct MR diff line"
    why_human: "No real GitLab reachable"
  - test: "Invalid position status code (expect 400) and fallback general note"
    expected: "Finding appears as general note with file and line"
    why_human: "Real wire format"
  - test: "glab 1.128.0 api -X POST -H Content-Type: application/json --input through internal CA"
    expected: "No 415, body accepted"
    why_human: "Real glab and CA"
  - test: "/merge, /approve, @user in findings"
    expected: "No quick action, no notification"
    why_human: "GitLab-side rendering"
  - test: "Status note queued -> result"
    expected: "Same note edited in place"
    why_human: "Real instance"
  - test: "Push during publication; kill server mid-publication then retry"
    expected: "Abort with nothing stale; no duplicates"
    why_human: "Real concurrency"
  - test: "Token without api scope"
    expected: "Actionable 403, no token in logs"
    why_human: "Real instance"
---

# Phase 4: Verification Report

**Goal:** A validated review appears on the MR as inline discussions, status note and summary, without duplicates or stale positions.
**Status:** human_needed (no gaps found in code; only real-GitLab checks remain, listed in 04-HUMAN-UAT.md)

## Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Inline discussion on right line (added/removed/context) | VERIFIED (fake) | `gitlab/diffs.rs` (old/new line rule), `publish.rs` posts position from versions[0]; `happy_path_posts_positions_then_summary_last` |
| 2 | Rejected position -> general note with same marker | VERIFIED (fake) | `rejected_position_falls_back_to_general_note_with_fingerprint`, `non_diffnote_is_edited_in_place`, `line_outside_the_diff_goes_straight_to_a_general_note` |
| 3 | Status note once, edited in place; summary note with suggestions | VERIFIED (fake) | `status_note_is_posted_once_then_edited_in_place`; `StatusCommentApi::edit` (default Ok(false) for GitHub, GitLab override in forge/mod.rs) |
| 4 | Retry/republish no duplicates | VERIFIED (fake) | `existing_fingerprints_and_summary_are_not_reposted`, `failed_inline_stops_before_summary_and_rerun_resumes`, `existing_diffnote_suppresses_same_fingerprint` |
| 5 | Push after review aborts; no quick actions/mentions/links | VERIFIED (fake) | `stale_head_aborts_before_any_write`, `head_change_before_summary_aborts_without_summary`; `finalize_body` used on every body (notes.rs, publish.rs), ~15 sanitizer unit tests |

## Project constraints

- GitHub regression: `edit` defaults to Ok(false), `delete` kept; forge wrappers pass through; request_changes remap and approve rejection gated on `Forge::GitLab` only (review/findings.rs:134, review_http.rs:49).
- Writes use `glab api -X POST -H Content-Type: application/json --input <0600 tmpfile>` (write.rs:54-55); body/token not in argv (test asserts argv log).
- Never approve/merge/push: no such endpoints in gitlab/publish.rs or notes.rs; event ignored (`suggestion_fence_is_neutralised_and_event_ignored`).
- File sizes: all gitlab files <= 396 lines, forge/mod.rs 376, publication.rs 1447 (was 1454, shrank), all < 2500.
- Changelog fragment `changelog.d/feat-gitlab-connection.md` covers publication.
- "Phase 4" stub gate string no longer present in backend/src.

## Requirements coverage

PUB-01..PUB-07 all claimed by plans (04-01: 01,02,07; 04-02: 03,06; 04-03/04-04: all) and mapped in REQUIREMENTS.md; none orphaned. Code evidence present for all seven; real-GitLab confirmation pending. Note: REQUIREMENTS.md checkboxes/table still show PUB-03..06 unchecked and all "Pendente"; bookkeeping to update after UAT.

## Anti-patterns

None found (no TBD/FIXME markers checked via targeted review of listed files; not exhaustive).

## Not checked

Full suite not rerun (executor reported 730 passed); no exhaustive debt-marker grep; real GitLab behavior unverified.
