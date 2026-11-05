---
phase: 05-indexa-o-skills-ui-e-documenta-o
reviewed: 2026-11-05T00:00:00Z
depth: standard
files_reviewed: 24
files_reviewed_list:
  - README.md
  - backend/src/gitlab/auth.rs
  - backend/src/server/indexing.rs
  - backend/src/server/mod.rs
  - backend/src/server/tests.rs
  - changelog.d/feat-gitlab-connection.md
  - docs/gitlab.md
  - src/platform/components/ReviewFlowNodes.tsx
  - src/platform/lib/forgeTerms.test.tsx
  - src/platform/lib/forgeTerms.ts
  - src/platform/pages/ReviewsPage.tsx
  - src/platform/pages/SettingsPage.tsx
  - src/platform/pages/reviews/ReviewDetail.tsx
  - src/platform/pages/reviews/ReviewSections.test.tsx
  - src/platform/pages/reviews/ReviewSections.tsx
  - src/platform/pages/reviews/RunDetails.tsx
  - src/platform/pages/settings/ReviewSettingsPanel.tsx
  - src/platform/pages/usage/PrUsageList.tsx
  - src/platform/pages/usage/UsageBreakdown.tsx
  - src/platform/pages/usage/UsageHero.tsx
  - src/platform/pages/usage/UsageTotals.tsx
findings:
  critical: 0
  warning: 2
  info: 2
  total: 4
status: issues_found
---

# Phase 5: Code Review Report

**Reviewed:** 2026-11-05
**Depth:** standard

## Summary

No GitHub regressions found. GitHub strings, `#n`, and the GitHubMark are unchanged when `forge === "github"`. All `useForgeTerms()` calls sit at the top level of components and run before any early return. `clone_url` is correct: the GitLab host comes from `forge::gh_host()`, nested paths are preserved, and git auth goes through `forge::apply_git_https_auth`. No token appears in the docs or error strings. `docs/gitlab.md` matches `docker-compose.yml` (`OCR_FORGE`, `OCR_GITLAB_HOST`, `OCR_GITLAB_CA_FILE` mounted at `/run/ocr-ca/ca.crt:ro`) and the `docker/gitlab-spike.py` flags and `GITLAB_TOKEN` env handling.

## Warnings

### WR-01: `terms` object in `useMemo` deps defeats memoization

**File:** `src/platform/pages/ReviewsPage.tsx` (the filter `useMemo`, deps `[..., terms]`)
**Issue:** `forgeTerms()` returns a new object on every render, so the memo recomputes every render. It also adds no safety.
**Fix:** Depend on `terms.forge` and call `forgeTerms(terms.forge).ref`, or memoize inside `useForgeTerms` with `useMemo(() => forgeTerms(kind), [kind])`. The second option fixes every consumer.

### WR-02: Forge terms flash or stick on GitHub wording for GitLab deploys

**File:** `src/platform/lib/forgeTerms.ts:28-38`
**Issue:** While `forgeStatus` is pending, or if it fails (`retry: false`, no refetch because `staleTime` is infinite), a GitLab deploy shows "pull request", `#n` and the GitHub wording. A failed first fetch means the wrong terms for the whole session. The per-session failure is the real risk, because the wrong-forge text stays on screen until reload.
**Fix:** Let a failed query retry (for example `retry: 2`, or no `staleTime: Infinity` while `data` is undefined). Optionally fall back to a neutral value until the status resolves.

## Info

### IN-01: Hardcoded GitHub text kept behind `verdicts`

**File:** `src/platform/pages/reviews/ReviewDetail.tsx:354,367`
**Issue:** The "GitHub blocks self-approval" and "Approve on GitHub" strings are hardcoded. They are only reachable when `verdicts` is true, so this is correct today. It becomes wrong if `verdicts` is ever enabled for GitLab.
**Fix:** Use `terms.platform` in these strings.

### IN-02: Stale comment in the reindex worker

**File:** `backend/src/server/indexing.rs:~384`
**Issue:** The comment "Ask GitHub whether the default branch moved" is now forge-neutral. Same for the "gh token" clone comment near line 436.
**Fix:** Reword to "the forge" and "forge token".

---

_Reviewed: 2026-11-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
