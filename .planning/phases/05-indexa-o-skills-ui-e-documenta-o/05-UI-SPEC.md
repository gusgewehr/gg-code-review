---
phase: 05
slug: indexa-o-skills-ui-e-documenta-o
status: approved
reviewed_at: 2026-11-05
shadcn_initialized: false
preset: none
created: 2026-11-05
---

# Phase 05 — UI Design Contract

Scope: vocabulary swap only. In a GitLab deploy, the Reviews page, review detail, run details, usage and sign-in recovery say "merge request"/"MR", `!123`, plain text "GitLab" with a generic lucide icon. No new layout, color, spacing, typography or component. GitHub deploys: every string, link and the GitHub mark stay byte-identical. All decisions are Claude's choice (autonomous). Baselines: `02-UI-SPEC.md`, `03-UI-SPEC.md`.

## Design System

| Property | Value |
|----------|-------|
| Tool | none (shadcn gate N/A) |
| Preset | not applicable |
| Component library | in-house (`DesignSystem.tsx`); unchanged |
| Icon library | lucide-react. GitLab deploy uses `GitMerge` at `size-4` in place of `GitHubMark` (no brand logo) |
| Font | unchanged |

Spacing, typography and color: unchanged, inherited from the existing elements. Accent usage unchanged. Exceptions: none.

## Forge source and helper

- Forge comes from the existing `GET /forge/status` (`ForgeStatus.provider`, `setupApi.forgeStatus`, query key `["forge","status"]`, already read in `SettingsPage.tsx`). Reuse it; do not add a second endpoint. If CONTEXT's `forge`/`forgeHost` fields are needed, add them to `ForgeStatus` only. While loading or on error, default to `github` (GitHub text never flickers to GitLab text wrongly for GitHub deploys).
- New module `src/platform/lib/forgeTerms.ts` (not inside `ReviewsPage.tsx` or `reviewFlow.ts`, near the 800-line cap):
  - `forgeTerms(forge: "github" | "gitlab")` returns the strings in the table below plus `ref(n)` (`#n` or `!n`).
  - Small hook `useForgeTerms()` wrapping the forge-status query (stale time long; `retry: false`).
- `GitHubMark` is rendered only when forge is `github`; for `gitlab` render `<GitMerge className="size-4" />` (same wrapper, same classes).
- Links: use URLs returned by the backend (`pr.url`/`htmlUrl` already carried by the data); never build `github.com` URLs in the frontend. In GitLab, "Open on GitHub" opens the MR URL on the GitLab instance.

## Copywriting Contract

GitHub string (unchanged for GitHub deploys) -> GitLab string. Locations are where the string lives today.

| Location | GitHub string | GitLab string |
|----------|---------------|---------------|
| `ReviewsPage.tsx:524` search placeholder | Search PRs, repos, authors… | Search MRs, repos, authors… |
| `ReviewsPage.tsx:377` search match | `#${number}` | `!${number}` (also keep matching the bare number) |
| `ReviewsPage.tsx:566` empty | No pull requests match your search. / Poll GitHub to find pull requests. | No merge requests match your search. / Poll GitLab to find merge requests. |
| `ReviewsPage.tsx:775` empty | Poll GitHub, then select a pull request to inspect its process. | Poll GitLab, then select a merge request to inspect its process. |
| `ReviewsPage.tsx:624`, `reviews/ReviewSections.tsx:100,193` number | `#{number}` | `!{number}` |
| `ReviewsPage.tsx:656`, `ReviewDetail.tsx:135` aria-label/title | Open on GitHub | Open on GitLab |
| `ReviewsPage.tsx:659`, `ReviewDetail.tsx:138` icon | GitHubMark | lucide `GitMerge` |
| `ReviewDetail.tsx:92` | Share findings without a GitHub verdict. | Share findings without a GitLab verdict. |
| `ReviewDetail.tsx:99` | Approve this pull request on GitHub. | Approve this merge request on GitLab. |
| `ReviewDetail.tsx:106` | Close locally without posting to GitHub. | Close locally without posting to GitLab. |
| `ReviewDetail.tsx:241` | Stacked PR | Stacked MR |
| `ReviewDetail.tsx:250` | …comments target only this PR's delta. | …comments target only this MR's delta. |
| `ReviewDetail.tsx:307` | View on GitHub | View on GitLab |
| `ReviewDetail.tsx:351` | This is your PR. GitHub blocks self-approval and request-changes; mark reviewed locally or skip. | This is your MR. GitLab blocks self-approval; mark reviewed locally or skip. |
| `ReviewDetail.tsx:360-362` | …to GitHub… / Approve on GitHub… | …to GitLab… / Approve on GitLab… |
| `reviews/RunDetails.tsx:55,138,154` | PR total / Full PR / Full PR vs base | MR total / Full MR / Full MR vs base |
| `usage/UsageTotals.tsx:40` | PR / PRs | MR / MRs |
| `usage/UsageTotals.tsx:46` | …published to GitHub. | …published to GitLab. |
| `usage/PrUsageList.tsx`, `UsagePage` tab | Pull requests | Merge requests |
| `SettingsPage.tsx` section "PR selection", `Maximum PR files`, `All PRs` | PR selection / Maximum PR files / All PRs | MR selection / Maximum MR files / All MRs (labels only; setting keys unchanged) |
| Sign-in error / recovery (GitLab) | GitHub authorization, Connect GitHub, Open GitHub verification page | Not rendered on GitLab (already hidden by Phase 2). Auth errors say: `GitLab sign-in failed. Check that glab is installed and the personal access token (scope api) is valid and not expired, then reconnect in GitLab connection.` |

Rules:
- Strings never contain a token or raw API payload.
- Strings not listed (e.g. `usage/shareCard.ts`, `useReviewRunStream.ts`, `Markdown.tsx`, `findings.ts`, `ReviewFlowNodes.tsx`, `ReviewSettingsPanel.tsx`) must be checked by the executor with `rg -i "pull request|\bPRs?\b|GitHub" src` and routed through `forgeTerms` if user-visible in a GitLab deploy. Code comments and identifiers (`prId`, `getPrUsage`) are not renamed.
- Primary CTA, empty and error states: those above. Destructive actions: none new (existing sign-out confirm stays; GitLab variant is already Phase 2).

## Tests (Vitest)

One test per surface: GitHub forge output equals the current strings (snapshot of `forgeTerms("github")`); GitLab forge shows `!n`, "merge request", no `GitHubMark` svg. Existing tests keep passing unchanged (default forge is `github`).

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | none | not applicable |
| third-party | none | not applicable |

## Checker Sign-Off

- [x] Dimension 1 Copywriting: PASS
- [x] Dimension 2 Visuals: PASS
- [x] Dimension 3 Color: PASS
- [x] Dimension 4 Typography: PASS
- [x] Dimension 5 Spacing: PASS
- [x] Dimension 6 Registry Safety: PASS

**Approval:** pending
