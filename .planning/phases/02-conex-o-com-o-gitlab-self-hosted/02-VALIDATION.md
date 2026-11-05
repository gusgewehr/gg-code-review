---
phase: 2
slug: conex-o-com-o-gitlab-self-hosted
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-11-05
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust, Postgres via `OCR_TEST_DATABASE_URL`), Vitest 4.1 + jsdom (frontend) |
| **Config file** | `Cargo.toml`, `vite.config.ts` (existing, no new framework) |
| **Quick run command** | `wsl -e bash -lc '~/rustcheck.sh "cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings"'` (backend); `npx -y pnpm@10.15.0 test -- GitLab` (frontend) |
| **Full suite command** | `wsl -e bash -lc '~/rusttest.sh'`; `npx -y pnpm@10.15.0 test`; `node scripts/check-file-sizes.mjs` |
| **Estimated runtime** | rustcheck ~2-5 min; rusttest several minutes; vitest ~1 min |

Local environment: no cargo/Docker on Windows; WSL Ubuntu has Docker and the `~/rustcheck.sh` / `~/rusttest.sh` wrappers. Frontend baseline: 20 failures in `localStorage.clear()` under Node 25 are pre-existing, not regressions. Fake-`glab` tests are `#[cfg(unix)]` and only run under WSL.

---

## Sampling Rate

- **After every task commit:** rustcheck for backend tasks; the relevant vitest file for frontend tasks
- **After every plan wave:** rusttest + full pnpm test + `node scripts/check-file-sizes.mjs`
- **Before `/gsd:verify-work`:** full suite green (except the documented Node 25 baseline)
- **Max feedback latency:** ~300 seconds

---

## Per-Task Verification Map

| Requirement | Behavior | Test Type | Automated Command / Location | File Exists | Status |
|-------------|----------|-----------|------------------------------|-------------|--------|
| FORGE-01 | pure guard: same ok; provider/host change => Err naming value; gitlab without host => Err | unit | `forge` guard tests via rusttest | ❌ W0 | ⬜ pending |
| FORGE-01 | first boot writes keys; mismatch on second boot errs; legacy GitHub DB without keys passes | pg test | rusttest | ❌ W0 | ⬜ pending |
| FORGE-01 | `forge_provider`/`forge_host` not writable through POST /settings | unit | rusttest | ❌ W0 | ⬜ pending |
| AUTH-01/03 | login/status/logout against fake `glab` (PATH override, records args+stdin) | unit | `gitlab` auth tests via rusttest | ❌ W0 | ⬜ pending |
| AUTH-02 | error classifier: missing api scope, expired, 401, x509/TLS, unreachable, generic | unit | rusttest | ❌ W0 | ⬜ pending |
| AUTH-05 | token never in argv/env/response/log; glab stderr echoing token redacted | unit | rusttest | ❌ W0 | ⬜ pending |
| AUTH-04 | CA bundle concat; no skip-verify strings in backend/docker/compose | unit + grep | rusttest + grep | ❌ W0 | ⬜ pending |
| AUTH-05 | `Dockerfile.runner` has no glab; runner services get no CA/GitLab env; server Dockerfile pins glab sha256 | static check | grep / test | ❌ W0 | ⬜ pending |
| AUTH-01..03 | panel states, token cleared after submit, hidden on GitHub deploy | vitest | `npx -y pnpm@10.15.0 test` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Fake-`glab` shell helper for unix tests (tempdir script prepended to `PATH`)
- [ ] `ensure_setting_default` reachable from the forge guard (visibility change only)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Spike against the real GitLab instance (PAT format, pagination, CA, API version) | SC-1 | Instance not reachable from this machine | Run `docker/gitlab-spike.py` per `SPIKE.md`, fill in findings |
| Real glab/git trust the mounted internal CA | AUTH-04 | Needs the real instance or a throwaway-CA TLS server | Mount CA, Connect in Settings, confirm status shows user |
| Server image builds with pinned glab sha256 | AUTH-05 | Long Docker build | `wsl -e bash -lc 'cd /mnt/c/Users/gusge/Documentos/GitHub/opencodereview && docker build -f docker/Dockerfile .'` (optional) |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 300s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
