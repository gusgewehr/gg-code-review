---
phase: 02-conex-o-com-o-gitlab-self-hosted
plan: 01
subsystem: backend-boot
tags: [forge, guard, ca, gitlab]
requires: []
provides: [Forge::GitLab, forge::init_from_env, forge::guard::enforce, gitlab::ca::prepare]
affects: [server boot]
key-files:
  created: [backend/src/forge/guard.rs, backend/src/gitlab/mod.rs, backend/src/gitlab/ca.rs]
  modified: [backend/src/forge/mod.rs, backend/src/github/auth.rs, backend/src/github/mod.rs, backend/src/db/settings.rs, backend/src/server/mod.rs, backend/src/server/polling.rs, backend/src/server/routes.rs, backend/src/lib.rs]
requirements-completed: [FORGE-01, AUTH-04]
metrics:
  completed: 2026-11-05
---

# Phase 2 Plan 1: Forge selection, pin guard and internal CA Summary

Forge selected from OCR_FORGE/OCR_GITLAB_HOST and pinned in settings, GitLab deploys skip GitHub-only workers, and an internal CA from /run/ocr-ca/ca.crt is merged with system roots at boot.

## Override applied

Per the user's override, the GitHub host pinned by the guard comes from existing detection (OCR_GH_HOST, GH_HOST, `gh auth status`), not "OCR_GH_HOST or github.com".
- `github::auth::detected_gh_host() -> Option<String>`; cached `GH_HOST` OnceLock now holds `Option<String>`; `gh_host()` still returns `detected.unwrap_or("github.com")` (no GitHub flow change); `gh_host_detected()` exposed to `forge`.
- `forge::configured` takes the detected Option; GitHub with None records nothing and the boot guard is skipped (no refusal to start). GitLab path unchanged (OCR_GITLAB_HOST required/validated).
- Detection is skipped entirely on GitLab deploys.
- Plan checks superseded: the `gh_host()` grep and the `configured(None,None,None) == github.com` case were replaced by None => no host / Some("ghe.example.com") => pinned.

## Deviations

- One commit (8bd166e) instead of two: both tasks edit `server/mod.rs` and the boot wiring depends on both.
- Initial scripted edit failed on shell quoting; redone with file tools (no impact).

## Verification

- `cargo fmt --all` + `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- Full `rusttest.sh` run was killed by the system for low memory, so the new tests (forge configured/URL, guard incl. Postgres, ca) have NOT been run. Re-run `~/rusttest.sh "--lib forge"` and `"--lib gitlab"` when memory allows. `check-file-sizes` not run.

## Self-Check: PARTIAL (compiles and lints; tests unrun)
