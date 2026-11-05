---
phase: 05-indexa-o-skills-ui-e-documenta-o
reviewed: 2026-11-05T00:00:00Z
depth: standard
iteration: 2
files_reviewed: 4
files_reviewed_list:
  - src/platform/lib/forgeTerms.ts
  - src/platform/lib/forgeTerms.test.tsx
  - src/platform/pages/ReviewsPage.tsx
  - src/platform/pages/SettingsPage.tsx
findings:
  critical: 0
  warning: 0
  info: 2
  total: 2
status: issues_found
---

# Phase 5: Code Review Report (iteration 2)

**Reviewed:** 2026-11-05
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found (Info only)

## Summary

Both iteration 1 warnings are resolved after commit 1d94867, with no regressions found.

- WR-01 is resolved. `useForgeTerms` now returns the module-level `GITHUB_TERMS` or `GITLAB_TERMS` constant, so the reference is stable and `terms` in `useMemo` deps behaves correctly.
- WR-02 is resolved. The hook query uses `retry: 2`, so a transient error no longer pins GitHub wording. With `staleTime: Infinity`, a query with no data still counts as stale. A failed query therefore refetches on the next observer mount or refocus.
- The shared key `["forge","status"]` has differing options in `SettingsPage` (`retry: false`, default `staleTime`) and in the hook. Both observers share one query and one `queryFn`. The worst case is fewer retries when `SettingsPage` triggers the fetch. That is harmless and not a bug.
- GitHub text is unchanged. The default path returns `forgeTerms("github")`, which matches the earlier output.
- Hook order is intact. `useContext` and `useQuery` are called unconditionally. The idle client plus `enabled: false` keeps the hook valid without a provider.

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
