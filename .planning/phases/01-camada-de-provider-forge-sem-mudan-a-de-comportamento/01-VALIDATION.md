---
phase: 1
slug: camada-de-provider-forge-sem-mudan-a-de-comportamento
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-11-05
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust), vitest (frontend, unchanged) |
| **Config file** | `Cargo.toml`, `vite.config.ts` |
| **Quick run command** | grep gate (below) + `cargo check --workspace --locked --all-targets` when cargo is available |
| **Full suite command** | `cargo test --workspace --locked` (needs `OCR_TEST_DATABASE_URL`), `cargo clippy --workspace --all-targets --locked -- -D warnings`, `pnpm test`, `pnpm check:file-size` |
| **Estimated runtime** | grep ~1s; cargo suite several minutes |

Local environment (2026-11-05): cargo, rustc, pnpm, docker not on PATH (Git Bash and PowerShell); `OCR_TEST_DATABASE_URL` unset. Only the grep gate and `node scripts/check-file-size.mjs` (if it runs without pnpm) run locally; cargo/pnpm/Docker gates run in CI.

---

## Sampling Rate

- **After every task commit:** grep gate
- **After every plan wave:** grep gate + file-size check
- **Before `/gsd:verify-work`:** full suite in CI must be green
- **Max feedback latency:** 5 seconds (local grep)

Grep gate (must print nothing):
`grep -rn "github::" backend/src --include=*.rs | grep -v "^backend/src/github/" | grep -v "^backend/src/forge/"`

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 1-01-* | 01 | 1 | FORGE-02 | — | No credential surface change | grep + unit | grep gate; `cargo test -p opencodereview-server forge` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| GitHub PR reviewed and published with identical bodies/URLs | FORGE-02 | Needs Docker deploy and a real PR | Rebuild server+runner images, review a PR, compare comment bodies/URLs with a pre-refactor run |
| Docker smokes unchanged | FORGE-02 | Docker unavailable locally | Run `docker/runner-smoke.py` and other smokes in a Docker host |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 5s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-11-05
