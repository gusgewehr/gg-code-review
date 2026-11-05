---
phase: 05-indexa-o-skills-ui-e-documenta-o
verified: 2026-11-05T00:00:00Z
status: human_needed
score: 4/4 must-haves verified in code (live-instance checks pending)
human_verification:
  - test: "Index a real GitLab project (nested subgroup) with OCR_AUTO_REINDEX=true"
    expected: "Clone succeeds with the CA, metadata, package graph and symbols appear"
    why_human: "No GitLab instance reachable"
  - test: "Open a GitLab deploy UI and click Open on GitLab"
    expected: "Text says merge request / !iid, link opens the MR on the instance"
    why_human: "Needs live instance and browser"
  - test: "Run docker/gitlab-spike.py command from docs/gitlab.md"
    expected: "Command works as documented"
    why_human: "Needs live instance"
---

# Phase 5 Verification

**Status:** human_needed (no code gaps found)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Indexing works for GitLab (IDX-01) | VERIFIED in code | `clone_url(host, full_name)` in `backend/src/server/indexing.rs:463`, used at line 439 with `forge::gh_host()`, which returns the GitLab host on GitLab. Reindex worker started at `server/mod.rs:291` gated only on `auto_reindex`, not on forge. Unit test in `server/tests.rs:1574` covers GitHub and nested GitLab path. Git auth goes through `forge::apply_git_https_auth`, which dispatches to `gitlab::apply_git_auth`. Real run not done. |
| 2 | Skills work for GitLab (IDX-02) | VERIFIED (orchestrator-reported tests, 05-01 plan) | Plan 05-01 proves skill code is forge-agnostic. I did not re-audit skill code beyond the plan and tests. |
| 3 | UI says merge request / `!iid`, links open the instance (UI-01) | VERIFIED in code | `src/platform/lib/forgeTerms.ts` and `useForgeTerms` are used in 12 components (reviews, flow, usage, settings). `terms.verdicts` is false on GitLab, so Approve and Request changes are disabled. Links use backend URLs. No github.com literals remain in non-test src files. One caveat: the ReviewSections test fails only because of the pre-existing localStorage baseline problem. |
| 4 | Docs and release note (UI-02) | VERIFIED | `docs/gitlab.md` (67 lines) covers OCR_FORGE, OCR_GITLAB_HOST, OCR_GITLAB_CA_FILE mounted at /run/ocr-ca/ca.crt, PAT scope `api`, Settings login, spike script, limitations. README links to it. `changelog.d/feat-gitlab-connection.md` has indexing/skills and UI vocabulary bullets. No token patterns found in docs or changelog. |

## Requirements Coverage

| ID | Status |
|----|--------|
| IDX-01 | SATISFIED in code, live check pending |
| IDX-02 | SATISFIED |
| UI-01 | SATISFIED in code, live check pending |
| UI-02 | SATISFIED |

All 4 IDs appear in plans and REQUIREMENTS.md. No orphaned requirements. REQUIREMENTS.md still marks them "Pendente" and unchecked, so the orchestrator should update it.

## Anti-patterns
None found. The 21 Vitest failures are the pre-existing baseline plus one test that fails through the baseline localStorage issue (per orchestrator).

## Gaps
None. Human verification covers the live-instance items above.

## Final gate (2026-11-05, HEAD 3aaa8f1)

| Check | Result |
|-------|--------|
| `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| Filtered Rust: `--lib gitlab` / `indexer` / `server::` / `clone_url` | 74 / 16 / 92 / pass |
| Full `rusttest.sh` (postgres:17) | 735 passed, 0 failed, 15 ignored (baseline 734 + new `clone_url` test) |
| `pnpm test` | 282 passed, 21 failed, 7 skipped. 20 = known baseline (Node 25 `localStorage.clear()` and CRLF-checkout `seo.test.ts`); 21st = new `ReviewSections.test.tsx > shows !n instead of #n on GitLab`, which fails only via the file's pre-existing `afterEach(localStorage.clear())` and passes with `--localstorage-file` |
| `tsc --noEmit -p .` | clean |
| `biome lint src` | clean (phase-5 files also pass `biome format` on LF content) |
| `node scripts/check-file-sizes.mjs` | pass (312 files) |
| `pnpm changelog:check` | valid |

Code review: 2 passes; WR-01/WR-02 fixed (1d94867), 0 critical/warning left, 2 info (IN-01 ReviewDetail strings only reachable with verdicts, IN-02 stale comments). UI audit: 21/24, sign-out and concurrency copy routed through forge terms (559e088). Human checks: see 05-HUMAN-UAT.md.
