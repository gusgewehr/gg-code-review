---
phase: 4
slug: escrita-gitlab-publica-o-dos-resultados
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-11-05
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust 1.98.1), inline `#[test]`, fake `glab` shell scripts (`#[cfg(unix)]`) |
| **Config file** | `backend/Cargo.toml`; DB tests need `OCR_TEST_DATABASE_URL` (via `~/rusttest.sh`) |
| **Quick run command** | `wsl -e bash -lc '~/rusttest.sh "--lib gitlab"'` |
| **Full suite command** | `wsl -e bash -lc '~/rusttest.sh'` |
| **Estimated runtime** | ~900 seconds (full, 1 build job) |

---

## Sampling Rate

- **After every task commit:** Run `--lib gitlab` (+ `--lib forge` when `forge/mod.rs` changes)
- **After every plan wave:** Run `--lib server::` and `--lib github`
- **Before `/gsd:verify-work`:** Full suite, clippy, fmt, file-size must be green
- **Max feedback latency:** 600 seconds

---

## Per-Task Verification Map

| Req | Behavior | Test Type | Automated Command | File Exists | Status |
|-----|----------|-----------|-------------------|-------------|--------|
| PUB-01 | position for added/removed/context lines; DiffNote asserted | unit + fake glab | `--lib gitlab` | ❌ W0 | ⬜ pending |
| PUB-02 | 400 or non-DiffNote falls back to general note with marker | fake glab | `--lib gitlab` | ❌ W0 | ⬜ pending |
| PUB-03 | status note created once then PUT in place; GitHub unchanged | unit + fake glab | `--lib server::status`, `--lib gitlab` | ❌ W0 | ⬜ pending |
| PUB-04 | summary note last with marker; suggestions section; returns note id | fake glab | `--lib gitlab` | ❌ W0 | ⬜ pending |
| PUB-05 | repost skips existing fingerprints; partial-failure retry posts only rest | fake glab | `--lib gitlab` | ❌ W0 | ⬜ pending |
| PUB-06 | MR head mismatch aborts before any POST | fake glab | `--lib gitlab` | ❌ W0 | ⬜ pending |
| PUB-07 | sanitizer cases; no body/token in argv | unit | `--lib gitlab` | ❌ W0 | ⬜ pending |
| Regression | GitHub flow untouched | existing | `--lib forge`, `--lib github`, `--lib server::` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `backend/src/gitlab/publish_tests.rs` — fake glab with body capture
- [ ] sanitizer tests (inline or `sanitize_tests.rs`)
- [ ] extend status-comment test fake with `edit` / `delete(number)`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Real `diffs`/`versions` shape, invalid-position status code | PUB-01/02 | No reachable GitLab instance | Publish a review on a real MR; check inline placement and fallback |
| `glab api --input` + Content-Type on glab 1.128.0 | PUB-01..04 | Needs real glab + instance | Run publish against self-hosted GitLab with internal CA |
| Quick actions inside code fences not executed | PUB-07 | GitLab server behaviour | Post finding containing `/merge` in a fence; confirm MR not merged |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 600s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
