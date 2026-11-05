---
phase: 3
slug: leitura-gitlab-descoberta-e-prepara-o-de-mrs
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-11-05
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust, Postgres via `OCR_TEST_DATABASE_URL` from `~/rusttest.sh`), Vitest 4.1 + jsdom |
| **Config file** | `backend/Cargo.toml`, `vite.config.ts` (existing) |
| **Quick run command** | `wsl -e bash -lc '~/rusttest.sh "--lib gitlab"'` (backend); `npx -y pnpm@10.15.0 test -- RepositorySelector utils` (frontend) |
| **Full suite command** | `wsl -e bash -lc '~/rusttest.sh'`; `npx -y pnpm@10.15.0 test`; `node scripts/check-file-sizes.mjs` |
| **Estimated runtime** | rusttest filtered ~3-5 min (build), full suite much longer; vitest ~1 min |

Baseline: 20 frontend failures from `localStorage.clear()` under Node 25 are pre-existing. Fake-`glab` tests are `#[cfg(unix)]` and run only under WSL. `forge::current()` is a process OnceLock (GitHub in tests): test GitLab arms through `gitlab::` functions directly.

---

## Sampling Rate

- **After every task commit:** `~/rusttest.sh "--lib gitlab"` for backend tasks (or the module filter touched); relevant vitest file for frontend tasks
- **After every plan wave:** fmt + clippy, filtered rusttest for touched modules, full pnpm test, file-size check
- **Before `/gsd:verify-work`:** full suite green (except the Node 25 baseline)
- **Max feedback latency:** ~300 seconds

---

## Per-Task Verification Map

| Requirement | Behavior | Test Type | Automated Command / Location | File Exists | Status |
|-------------|----------|-----------|------------------------------|-------------|--------|
| FORGE-03 | `Scope::validate` accepts `g/sub/proj`, rejects `a`, `a//b`, `..`; `rsplit_once` owner/name; GitHub two-segment unchanged | unit | `~/rusttest.sh "--lib review_archive"`, `"--lib db::repositories"` | extend existing | ⬜ pending |
| REPO-01 | project fixtures -> workspaces + `RepoInput`; paginated output shapes (array, concatenated arrays, NDJSON) | unit | `~/rusttest.sh "--lib gitlab"` | ❌ W0 | ⬜ pending |
| REPO-02/03 | MR fixture -> `GhPullRequest`; `draft`/`work_in_progress` -> `is_draft` | unit | `~/rusttest.sh "--lib gitlab"` | ❌ W0 | ⬜ pending |
| REPO-04 | state opened/closed/merged/locked mapping for lifecycle | unit | `~/rusttest.sh "--lib gitlab"` | ❌ W0 | ⬜ pending |
| REPO-05 | head change supersedes via existing reset path | unit | existing `server::` tests | exists | ⬜ pending |
| REV-01 | git auth extraheader via `GIT_CONFIG_*` env only; token never in args/errors | unit | `~/rusttest.sh "--lib gitlab"` | ❌ W0 | ⬜ pending |
| REV-02/03 | `pr_head_ref` for GitLab = `refs/merge-requests/<iid>/head`; live MR state via fake glab | unit | `~/rusttest.sh "--lib gitlab"`, `"--lib forge"` | partial | ⬜ pending |
| UI | `repoOwner` nested namespaces; RepositorySelector GitLab copy | vitest | `npx -y pnpm@10.15.0 test` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `backend/src/gitlab/*_tests.rs` with JSON fixtures and fake-glab helper (reuse `auth_tests.rs` pattern)
- [ ] frontend tests for `repoOwner` and RepositorySelector GitLab copy

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Real `glab api --paginate` output shape, field names (`draft`, `sha`, `namespace.full_path`) | REPO-01..04 | No GitLab instance reachable | Run `docker/gitlab-spike.py`; connect, select a nested project, open an MR, watch it appear |
| Fetch of `refs/merge-requests/<iid>/head` with token extraheader and internal CA | REV-01..03 | Needs real instance | Open MR (incl. from a fork), confirm review prepares and diff is complete |
| glab `config.yml` token layout | REV-01 | glab version-specific | Confirm git fetch authenticates after Connect |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 300s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
