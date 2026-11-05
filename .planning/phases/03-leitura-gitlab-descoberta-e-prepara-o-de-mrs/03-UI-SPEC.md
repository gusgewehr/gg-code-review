---
phase: 03
slug: leitura-gitlab-descoberta-e-prepara-o-de-mrs
status: approved
shadcn_initialized: false
preset: none
created: 2026-11-05
---

# Phase 03 — UI Design Contract

Scope: one surface, Settings > Repository access, in a GitLab deploy (`OCR_FORGE=gitlab`). It lists GitLab namespaces (groups/subgroups) and their projects by REUSING `src/platform/pages/settings/RepositorySelector.tsx` and the existing organization picker. No new components, tokens, scales or sizes. GitHub deploys: UI unchanged (copy below applies only when forge is GitLab). All decisions are "Claude's choice" (autonomous mode); baseline is `02-UI-SPEC.md`.

## Design System

| Property | Value |
|----------|-------|
| Tool | none (shadcn gate N/A) |
| Preset | not applicable |
| Component library | in-house: `RepositorySelector`, `DSInput`, `DSEmptyState` (`src/components/DesignSystem.tsx`), `BrandLoading`; Settings controls in `SettingsControls.tsx` |
| Icon library | lucide-react (`Search` `size-4`, already used) |
| Font | `--font-sans`; `--font-mono` unchanged |

## Data Mapping (Claude's choice)

Reuse the existing `Repo` shape; no frontend type change beyond what the API already returns.

| UI concept | GitLab source |
|------------|---------------|
| Organization / workspace | namespace `full_path` (group or subgroup, may have 3+ segments, e.g. `group/sub`) |
| Repo `fullName` | project `path_with_namespace` (e.g. `group/sub/proj`; shown whole, wraps via existing `[overflow-wrap:anywhere]`) |
| `defaultBranch` | project `default_branch` (existing fallback "Unknown branch") |
| `isPrivate` | project visibility `private` or `internal` => Private; `public` => Public |
| Listed projects | only those where the user has Developer+ access (access level >= 30) |

Constraints for the implementer: do not split `fullName` on `/` assuming exactly 2 segments. `repoOwner(repo)` (`src/platform/lib/utils`) must return the project's namespace (everything before the last `/`) in a GitLab deploy, so `locked` (selected namespace) matching and search by "organization" work for nested groups. Organization labels display the full path, never only the last segment.

## Spacing Scale

Existing values only, multiples of 4: `gap-3` (12px) list/column gap, `px-3.5 py-3` option padding (existing), `min-h-14`. Exceptions: none (inherited as-is from `RepositorySelector`).

## Typography

Unchanged, inherited from the component: `text-small font-medium` (repo name), `text-caption` (meta, counter), weights 400 and 500/600 as already rendered. No new sizes or weights.

## Color

| Role | Value | Usage |
|------|-------|-------|
| Dominant | `--color-canvas` / `--color-surface` | Page background |
| Secondary | `--color-line` border, `bg-surface-muted` hover | List container and rows |
| Accent | `--color-brand` | Checkbox accent, focus outline, selected row tint (existing only) |
| Destructive | `--color-danger` | Error message text only |

Accent reserved for: checkbox, focus ring, selected row (all existing). No new color usage.

## Layout and Interaction

- Identical layout to GitHub: label "Repositories" + "N of M selected", search input, scrollable checkbox list. Nested namespaces appear as flat entries by full path; no tree, no indentation (Claude's choice, minimal).
- Search matches `fullName` substring (existing behavior), so `group/sub` filters all projects under it.
- States: not connected, no namespaces selected, loading, no results, empty (no Developer+ projects), error. Each rendered with `DSEmptyState` / `BrandLoading` in the list box; error is a `text-sm font-semibold text-danger` line above the list, same pattern as Phase 2.
- Search placeholder, labels and `aria-label`s use forge-neutral wording only where the string would otherwise say GitHub-specific terms; in GitHub deploys the current strings stay byte-identical.

## Copywriting Contract

GitLab deploy only. GitHub strings are untouched.

| Element | Copy |
|---------|------|
| Section/primary action | Refresh repository access (existing label, unchanged) |
| Organizations helper (none selected) | Select groups above to browse their projects. |
| Search placeholder | Search by group or project name… |
| Loading | Loading projects… |
| Empty state (no Developer+ projects) | No projects available. GitLab only lists projects where you have Developer access or higher. Refresh repository access to check again. |
| Empty state (search) | No projects match this search. |
| Empty state (GitLab not connected) | GitLab is not connected. Connect with a personal access token in GitLab connection, then refresh repository access. |
| Error: list failed | Could not load projects from {host}. Check the connection and refresh repository access. |
| Error: token rejected/expired | GitLab rejected the stored token. Reconnect in GitLab connection with a new personal access token. |
| Unknown branch fallback | Unknown branch (existing) |
| Destructive actions | none in this phase (selection toggles are reversible; no dialog) |

Error strings must never contain the token or raw API payloads. "Repositories" label and "N of M selected" counter stay as-is (generic).

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | none | not applicable |
| third-party | none | not applicable |

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS

**Approval:** pending
