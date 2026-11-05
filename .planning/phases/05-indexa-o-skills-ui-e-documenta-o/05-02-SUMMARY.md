---
phase: 05-indexa-o-skills-ui-e-documenta-o
plan: 02
subsystem: frontend
tags: [gitlab, ui, vocabulary, react-query]
requires: [GET /forge/status (Phase 2)]
provides: [forgeTerms, useForgeTerms]
affects: [Reviews page, review detail, run details, review flow, usage, settings]
key-files:
  created:
    - src/platform/lib/forgeTerms.ts
    - src/platform/lib/forgeTerms.test.tsx
  modified:
    - src/platform/pages/ReviewsPage.tsx
    - src/platform/pages/reviews/ReviewDetail.tsx
    - src/platform/pages/reviews/ReviewSections.tsx
    - src/platform/pages/reviews/ReviewSections.test.tsx
    - src/platform/pages/reviews/RunDetails.tsx
    - src/platform/components/ReviewFlowNodes.tsx
    - src/platform/pages/usage/UsageTotals.tsx
    - src/platform/pages/usage/UsageHero.tsx
    - src/platform/pages/usage/PrUsageList.tsx
    - src/platform/pages/usage/UsageBreakdown.tsx
    - src/platform/pages/settings/ReviewSettingsPanel.tsx
    - src/platform/pages/SettingsPage.tsx
decisions:
  - "useForgeTerms shares the [forge, status] query key; defaults to GitHub when loading, errored or no QueryClientProvider (disabled idle client)"
  - "On GitLab, request_changes and approve_post decisions are disabled via terms.verdicts"
metrics:
  tasks: 3
  completed: 2026-11-05
---

# Phase 5 Plan 02: GitLab UI vocabulary Summary

`forgeTerms`/`useForgeTerms` swap PR to merge request / MR, `#n` to `!n`, GitHub to GitLab and GitHubMark to lucide `GitMerge` across review, flow, usage and settings surfaces; GitHub output stays byte-identical (default while loading/error/no provider).

## Commits
- 0196e82 `feat(05-02): add forgeTerms module and useForgeTerms hook`
- be778a4 `feat(05-02): route review surfaces through forge terms`
- 4dbcb9d `feat(05-02): route flow, usage and settings labels through forge terms`

## Deviations from Plan

**1. [Plan-directed] GitLab decisions**: approve_post and request_changes are disabled when `!terms.verdicts` (Phase 4 rejects them). Own-MR footer reads "This is your MR. Mark reviewed locally or skip."; no-findings footer reads "No findings. Mark reviewed locally, or skip." (differs from UI-SPEC rows, as the plan stated).

**2. [Rule 2 - consistency] Extra strings routed**: `PrUsageList` loading label, empty state and column header "Pull request" plus ReviewSettingsPanel "Title filters" description and GitHub teams text, and ReviewFlowNodes "in the PR" count, also use terms.

Links use backend URLs (`monitoring.url`, `card.url`); no forge URLs are built in the frontend. Files on this checkout are CRLF; edits preserved CRLF.

## Verification
- tsc, biome lint src, file-size check: pass.
- `vitest run src/platform`: 19 failed / 250 passed. Failures: 18 are `localStorage.clear/getItem is not a function` (Node 25 baseline), 1 is `ReviewsPage.decisions` "1.6k tokens" vs "1,6k" (locale formatting of token count, unrelated to this plan; that test is among the 2 that fail with the localStorage flag set too). The new GitLab `!1` test passes when localStorage works (run with `--localstorage-file`); it fails by default only through the `afterEach` localStorage.clear baseline.

## Residual GitHub/PR hits left as-is
- `lib/reviewFlow.ts` status details: pure non-React lib at the size cap.
- `usage/shareCard.ts`, `ShareUsage.tsx`: exported image totals ("Pull request").
- `AccountsPanel.tsx`, `AccessConnect.tsx`, `ErrorScreen.tsx`: onboarding/auth copy, GitHub sign-in hidden on GitLab by Phase 2.
- `GitHubAuthRecovery.tsx`, `GitHubMark.tsx`, `RepositorySelector.tsx`: GitHub-only components.
- `useReviewRunStream.ts`, `useReviewSelection.ts`, `findings.ts`, `Markdown.tsx`, `settingsForm.ts`, `PrRunsTimeline.tsx`, `RunStrip.tsx`, `PlatformLayout.tsx`, `ReviewFlow.tsx`, `DesignSystem.tsx`: identifiers/comments or static strings not reachable with PR vocabulary (not individually audited; risk).

## Known Stubs
None.

## Self-Check: PASSED
