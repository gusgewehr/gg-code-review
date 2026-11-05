---
phase: 02
slug: conexao-gitlab-self-hosted
status: approved
reviewed_at: 2026-11-05
shadcn_initialized: false
preset: none
created: 2026-11-05
---

# Phase 02 — UI Design Contract

Scope: one GitLab connection panel in Settings, mirroring the GitHub block. Rendered only when the deploy is GitLab (`OCR_FORGE=gitlab`); GitHub deploys see no UI change. No new tokens, scales or components: everything below reuses what exists.

## Design System

| Property | Value |
|----------|-------|
| Tool | none (project's own design system; shadcn not used, gate N/A) |
| Preset | not applicable |
| Component library | in-house: `DSButton`, `DSInput` (`src/components/DesignSystem.tsx`); `SettingsSection`, `SettingsGroup`, `SettingRow`, `TokenEditor`, `IntegrationRow` (`src/platform/pages/settings/SettingsControls.tsx`) |
| Icon library | lucide-react (`size-3.5` in buttons) |
| Font | `--font-sans` (UI), `--font-mono` (host, username) from `src/styles/globals.css` |

Implementation hint: build the panel from `SettingsSection` + `SettingRow`; use `TokenEditor` / `DSInput type="password"` for the PAT if it fits, else `DSInput`. Card chrome matches `GitHubAuthRecovery`: `grid gap-3 rounded-lg border border-line bg-[color-mix(in_srgb,var(--surface-2)_70%,transparent)] p-4`.

## Spacing Scale

Existing Tailwind 4 scale only (multiples of 4): `gap-3` (12px) inside the panel, `p-4` (16px) panel padding, `gap-4` (16px) between title block and action, `mt-1` (4px) title to description, `gap-3` between footer buttons. Exceptions: none.

## Typography

Existing tokens from `globals.css`, as used by `GitHubAuthRecovery`:

| Role | Size | Weight | Line Height |
|------|------|--------|-------------|
| Panel title (`text-sm font-semibold`) | 14px | 600 | Tailwind default |
| Body / description / status (`text-sm text-fg-muted`) | 14px | 400 | Tailwind default |
| Host, username, expiry values (`font-mono text-sm`) | 14px | 400 | Tailwind default |

Two sizes/weights in use: 14px only, weights 400 and 600. No new sizes.

## Color

| Role | Value | Usage |
|------|-------|-------|
| Dominant | `--color-canvas` #1b1b1b / `--color-surface` #1f1f1f | Page and input background |
| Secondary | `--surface-2` at 70% (panel card), `--color-line` border | Panel container |
| Accent | `--color-brand` #3b82f6 | Connect button (`tone="primary"`) only |
| Success | `--color-success` | "Connected" status text |
| Destructive | `--color-danger` #f87171 | Error messages; Disconnect text/tone |

Accent reserved for: Connect button only. Muted labels use `text-fg-muted`.

## Layout and Interaction

States of the panel (single card, title "GitLab connection"):

1. **Disconnected:** read-only Host row (`DSInput` disabled / mono text, value from deploy config, never editable); password input "Personal access token" (`type="password"`, `autoComplete="off"`, `spellCheck=false`); helper line "Needs the `api` scope."; footer right-aligned `Connect` (primary, disabled when field empty or loading; label `Connecting` while loading).
2. **Connected:** rows Host, User, Token expires (date, or "No expiry" when `expires_at` is null); success line "Connected"; footer `Disconnect` (default tone, danger text). No confirm dialog (reversible: paste token again).
3. **Error:** inline `text-sm font-semibold text-danger` line above footer, same pattern as `doneMessage` in `GitHubAuthRecovery`; panel stays in disconnected state with token field cleared.

Token handling: clear the input state immediately on submit (success or failure); never render, store in query cache, localStorage or URL; no show/hide toggle. Panel not rendered at all (no placeholder) when forge is GitHub.

## Copywriting Contract

| Element | Copy |
|---------|------|
| Panel title | GitLab connection |
| Panel description | Connect with a personal access token to review merge requests on {host}. |
| Primary CTA | Connect |
| Secondary CTA | Disconnect |
| Empty state (disconnected) | Not connected. Paste a personal access token with the `api` scope and select Connect. |
| Error: missing scope | Token is missing the `api` scope. Create a new token with `api` and try again. |
| Error: expired | This token has expired. Create a new token and try again. |
| Error: 401 | GitLab rejected the token (401). Check that it was copied in full and has not been revoked. |
| Error: unreachable / TLS | Could not reach {host} or verify its certificate. Check the host and that the internal CA is mounted for the server (`/run/ocr-ca/ca.crt`). |
| Error: generic | Could not connect to GitLab. Check the server logs for details. |
| Destructive confirmation | Disconnect: no dialog; removes the stored GitLab login for {host}. Reconnect by pasting a token again. |

Error strings must never contain the token.

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
