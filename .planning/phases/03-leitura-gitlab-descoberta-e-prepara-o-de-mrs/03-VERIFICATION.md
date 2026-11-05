---
phase: 03-leitura-gitlab-descoberta-e-prepara-o-de-mrs
verified: 2026-11-05T00:00:00Z
status: human_needed
score: 5/5 must-haves verified (code level); real-instance behavior pending HUMAN-UAT
overrides_applied: 0
human_verification:
  - test: "glab api output shape against a real GitLab (projects, merge_requests, draft/work_in_progress, 429 wording)"
    expected: "Repository access lists groups/subgroups/projects (Developer+); open MRs map to number=iid, sha, is_draft"
    why_human: "No reachable GitLab instance; parsing is covered only by fixtures"
  - test: "glab config.yml token layout (hosts:/<host>:/token:, !!null/quoted variants)"
    expected: "git_auth finds the token and git fetch authenticates; without token, fetch fails and preparation fails"
    why_human: "Layout flagged as 'shortcut: confirm in 02-SPIKE.md' in gitlab/git_auth.rs"
  - test: "Fetch refs/merge-requests/<iid>/head and target branch with internal CA, incl. a fork MR"
    expected: "Preparation succeeds with no fork credentials; diff computed locally for a large MR; a failing fetch fails preparation"
    why_human: "Needs real git transport with the internal CA"
  - test: "Push to an MR, close/merge an MR, toggle skip_draft_prs on a draft MR"
    expected: "New review supersedes previous; closed/merged MRs leave the queue; drafts follow the option"
    why_human: "End-to-end polling against a live instance"
  - test: "Nested project g/sub/proj: scope, retention and URLs end-to-end"
    expected: "No path or URL breakage in UI, archive and published links"
    why_human: "Only unit-level coverage exists"
---

# Phase 03 Verification Report

**Goal:** Open MRs of chosen GitLab projects appear in the review list, with the review prepared against correct frozen evidence.
**Status:** human_needed (no code gaps found; real-instance behavior is HUMAN-UAT by agreement)
**Re-verification:** No

## Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Admin sees groups/subgroups/projects, nested, and picks which to review | VERIFIED (code) | `gitlab/projects.rs` groups projects by `namespace.full_path`, `valid_full_name` allows N segments; `routes.rs` GitLab branch of refresh uses `gitlab::list_all` and upserts one workspace per namespace; `percent_decode` on `/workspaces/{login}/repos`; `repoOwner` in `src/platform/lib/utils.ts` uses `lastIndexOf`; RepositorySelector tests pass (2 files, 5 tests, run by me) |
| 2 | Open MRs appear via polling; drafts follow `skip_draft_prs`; closed/merged leave queue | VERIFIED (code) | `polling.rs`: `require_github` removed from `poll_reviews`; same `poll_one_pr` and `reconcile_pr_lifecycle` path; `gitlab/mrs.rs` maps `draft`/`work_in_progress` to `is_draft`, `map_state` to OPEN/CLOSED/MERGED, `live_pr_state` via forge |
| 3 | New push enqueues new review and supersedes old | VERIFIED (code) | `number=iid`, `head_ref_oid=sha` feed the unchanged `poll_one_pr` head-change logic; `live_pr_state` returns live MR sha |
| 4 | Fetch `refs/merge-requests/<iid>/head` with no checkout, live base/head/merge-base, fetch failure fails, local diff | VERIFIED (code) | `forge::Forge::GitLab.pr_head_ref` used by `review/git.rs::fetch_pr_head` and `summary.rs`; `fetch_base` propagates errors; clone/fetch auth via `forge::apply_git_https_auth` -> `gitlab::apply_git_auth` (GIT_CONFIG_* env, not argv) at `review/git.rs:458`, `indexer/mod.rs:620`, `summary.rs:245`; diff is local git |
| 5 | Fork MR without fork credentials; scope/retention/URLs for `g/sub/proj` | VERIFIED (code) | Fetch targets only the target project's `origin`; `review_archive` Scope validation changed to multi-segment (tests added in `review_archive/tests.rs`); retention path change in `retention.rs`; `frozen.rs` skips gh-only pinned-dependency resolution on GitLab |

Score: 5/5 at code level.

## Requirements Coverage

All 9 phase IDs appear in PLAN frontmatter and REQUIREMENTS.md: FORGE-03 (03-02, 03-03, 03-04), REPO-01 (03-01, 03-02, 03-04), REPO-02..05 (03-01, 03-04), REV-01 (03-01, 03-03), REV-02 and REV-03 (03-03). No orphans. All have code evidence; end-to-end confirmation is in human verification.

Note: the traceability table in `.planning/REQUIREMENTS.md` (lines 98-115) still says "Pendente" while the checklist is `[x]`. Cosmetic, update when closing the phase.

## Anti-Patterns

- No TBD/FIXME/XXX in `backend/src/gitlab` or `forge`.
- Warning (info): `gitlab/git_auth.rs` carries `shortcut: assumes glab config.yml layout`, which is unconfirmed (see human item 2).
- Warning (info): `require_github` fails closed in `github/transport.rs`, so any missed gh call on GitLab returns an error rather than shelling out; correct behavior.
- Info: GitLab author-command scanning is skipped by design; pinned-dependency evidence is skipped on GitLab (documented shortcut).

## Behavioral Checks

- Frontend: `vitest run` on `utils.test.ts` and `RepositorySelector.test.tsx` passed. The full suite shows 20 failures that SUMMARY 03-02 attributes to baseline (Node 25 localStorage); I did not re-confirm the baseline.
- Rust builds and tests were not run (per instruction). Relied on SUMMARY reports (gitlab 25, forge 7, server 86, review_archive 67, github 36 passing); not independently verified.

## Gaps Summary

No code gaps found. The only open items are the real-instance checks above.
