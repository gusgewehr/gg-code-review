# Phase 05 — UI Review

**Audited:** 2026-11-05
**Baseline:** 05-UI-SPEC.md
**Screenshots:** not captured (code-only audit, no dev server)

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Spec table swapped via `forgeTerms`; leftover GitHub/PR strings remain on GitLab |
| 2. Visuals | 3/4 | `GitMerge size-4` swap matches spec; `GitMerge` also used as a flow-node icon |
| 3. Color | 4/4 | No color changes introduced |
| 4. Typography | 4/4 | No typography changes introduced |
| 5. Spacing | 4/4 | No spacing changes introduced |
| 6. Experience Design | 3/4 | Loading defaults to GitHub text (spec-compliant) but GitLab deploys briefly flash GitHub wording; sign-out copy GitHub-only |

**Overall: 21/24**

## Top 3 Priority Fixes

1. **Sign-out confirm and settings nav still GitHub/PR wording on GitLab** — `SettingsPage.tsx:159` ("Sign out of GitHub?"), `settingsForm.ts:7` ("PR selection" nav title) and `ReviewSettingsPanel.tsx:53` ("How many pull requests...") are not routed through `forgeTerms`. The nav label conflicts with the "MR selection" group title in the same panel. Fix: use `terms` (nav titles may need a hook or a function taking terms).
2. **Hardcoded GitHub strings in `ReviewDetail.tsx:354,367`** ("This is your PR. GitHub blocks self-approval…", "Approve on GitHub…") — correct only while `verdicts` is false for GitLab; breaks if enabled (REVIEW IN-01). The spec's GitLab variant ("This is your MR. GitLab blocks self-approval…") was not implemented. Fix: use `terms.short` and `terms.platform`.
3. **Residual PR/GitHub copy in user-visible strings** — `reviewFlow.ts:48,247` ("PR exceeds the configured file limit", "changed PR head"), `AccountsPanel.tsx:439` ("review pull requests"). Spec rule requires an `rg` sweep. Fix: pass `terms` into `reviewFlow` helpers and the AccountsPanel text. Also, the spec's GitLab auth error string ("GitLab sign-in failed. Check that glab...") was not found in the audited code; confirm where it lives.

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- WARNING: items in Top 3 above.
- Done correctly: search/empty states (`ReviewsPage.tsx:568,777`), approve description (`ReviewDetail.tsx:101`), settings group labels (`ReviewSettingsPanel.tsx:27-44`), `ref(n)` helper.
- WARNING: `useForgeTerms` uses `retry: 2` while the spec says `retry: false` (deliberate, from review fix WR-02; harmless).

### Pillar 2: Visuals (3/4)
- `ReviewsPage.tsx:661` and `ReviewDetail.tsx:140` render `GitMerge className="size-4"` for GitLab and `GitHubMark` for GitHub, as specified. Link class `review-workspace-github-link` is a name only.
- WARNING: `GitMerge` is also the "merge" node icon in `ReviewFlowNodes.tsx:88`, so the same glyph means two things on GitLab. Minor; not in the spec.
- Not verified visually: no screenshots, so alignment of the swapped icon is unchecked (needs_human_review).

### Pillar 3: Color (4/4)
Scope was a vocabulary swap. The diff-bearing files show no new color classes.

### Pillar 4: Typography (4/4)
No new size or weight classes in the changed lines.

### Pillar 5: Spacing (4/4)
No new spacing classes. Icon sizing reuses `size-4`.

### Pillar 6: Experience Design (3/4)
- While `forge/status` loads or errors, the UI defaults to GitHub terms (spec requires this). A GitLab deploy will flash "pull request" on first paint. This is an accepted trade-off, but it is visible.
- Theme: the `idleClient` fallback keeps the hook safe without a provider. Hook order is correct.
- WARNING: `SettingsPage.tsx:159` sign-out confirm is GitHub-only, so a GitLab user sees the wrong platform in a destructive-action prompt.
- Tests: `forgeTerms.test.tsx` covers the GitHub snapshot and GitLab output. Per-surface GitLab tests were not examined.

## Files Audited
- src/platform/lib/forgeTerms.ts (+ test)
- src/platform/pages/ReviewsPage.tsx, SettingsPage.tsx
- src/platform/pages/reviews/ReviewDetail.tsx, ReviewSections.tsx, RunDetails.tsx
- src/platform/pages/settings/ReviewSettingsPanel.tsx, settingsForm.ts
- src/platform/pages/usage/*, src/platform/components/ReviewFlowNodes.tsx, AccountsPanel.tsx
- src/platform/lib/reviewFlow.ts

## Post-audit action

Fix 1 partially applied in 559e088: sign-out confirm and "Reviews at a time" description now use forge terms (the settings nav label was already routed at SettingsPage.tsx:383). Fixes 2-3 left as advisory: ReviewDetail lines are only reachable when verdicts are enabled (GitHub only); reviewFlow.ts / AccountsPanel copy stays generic per 05-02 SUMMARY.
