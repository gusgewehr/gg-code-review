---
phase: 05
slug: indexa-o-skills-ui-e-documenta-o
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-11-05
---

# Phase 05 — Validation Strategy

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Postgres via `~/rusttest.sh`), Vitest 4.1 + jsdom |
| **Config file** | `Cargo.toml`, `vite.config.ts` |
| **Quick run command** | `wsl -e bash -lc '~/rusttest.sh "--lib server::"'` ; `npx -y pnpm@10.15.0 exec vitest run src/platform/lib` |
| **Full suite command** | `wsl -e bash -lc '~/rusttest.sh'` ; `npx -y pnpm@10.15.0 test` |
| **Estimated runtime** | Rust full ~30-60 min (1 build job); Vitest ~60 s |

## Sampling Rate

- **After every task commit:** quick run for the touched side (Rust filter or Vitest file)
- **After every plan wave:** tsc, biome lint src, check-file-sizes, changelog:check
- **Before verify:** full suites green (Vitest: only the 20 baseline localStorage failures)

## Per-Task Verification Map

| Req | Behavior | Test Type | Automated Command | File Exists |
|-----|----------|-----------|-------------------|-------------|
| IDX-01 | reindex worker not gated on GitHub; clone URL uses forge host and nested path | unit | `~/rusttest.sh "--lib clone_url"` | ❌ W0 |
| IDX-01 | indexer tests unchanged | unit | `~/rusttest.sh "--lib indexer"` | ✅ |
| IDX-02 | skills forge-agnostic (no two-segment / github.com assumption) | grep + existing tests | `rg -n "github\.com" backend/src/server` | ✅ |
| UI-01 | `forgeTerms("github")` equals current strings; gitlab gives `!n`, "merge request" | vitest | `vitest run src/platform/lib/forgeTerms.test.ts` | ❌ W0 |
| UI-01 | GitLab render: no GitHub mark, link from backend URL | vitest | `vitest run src/platform/pages/reviews` | ❌ W0 |
| UI-02 | docs + release note present and valid | script | `pnpm changelog:check`; `test -f docs/gitlab.md` | ❌ W0 |

## Wave 0 Requirements

- [ ] `src/platform/lib/forgeTerms.test.ts`
- [ ] `clone_url` helper test in backend server tests

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Real GitLab project indexes (metadata, package graph, symbols) | IDX-01 | no reachable GitLab instance | Deploy with OCR_FORGE=gitlab, select a project, trigger reindex, open repository view |
| UI links open the GitLab instance | UI-01 | needs real MR data | Open a reviewed MR, click "Open on GitLab" |

## Validation Sign-Off

- [x] All tasks have automated verify or Wave 0 dependencies
- [x] No watch-mode flags
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-11-05
