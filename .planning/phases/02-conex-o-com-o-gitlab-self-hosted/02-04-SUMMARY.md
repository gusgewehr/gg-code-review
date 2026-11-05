---
phase: 02-conex-o-com-o-gitlab-self-hosted
plan: 04
subsystem: backend-auth
tags: [gitlab, glab, auth, routes]
requires: [02-01]
provides: [gitlab::login, gitlab::status, gitlab::logout, ForgeStatus, /forge/status, /auth/gitlab/token, /auth/gitlab/logout]
key-files:
  created: [backend/src/gitlab/auth.rs, backend/src/gitlab/auth_tests.rs]
  modified: [backend/src/gitlab/mod.rs, backend/src/github/mod.rs, backend/src/server/routes.rs, backend/src/server/tests.rs]
requirements-completed: [AUTH-01, AUTH-02, AUTH-03, AUTH-05]
metrics:
  completed: 2026-11-05
---

# Phase 2 Plan 4: glab auth and forge routes Summary

PAT login via `glab auth login --stdin --insecure-storage` (token on stdin only, scrubbed env, 20s deadline), status via `glab api user` + `personal_access_tokens/self`, classified UI-SPEC errors with rollback logout, and the three routes the 02-03 panel calls.

## Commit
- 1b95af7 feat(02-04): glab login/status/logout and forge HTTP routes (both tasks in one commit; they share one compile unit)

## Notes
- JSON shapes match src/types/settings.ts `ForgeStatus` and setupApiAccess.ts paths exactly; no frontend change.
- Routes are not public (behind OCR_API_TOKEN); POSTs require trusted origin. Token body parsed without serde error echo.
- Added `Debug` to ForgeStatus/StatusError (no secrets) so tests can `unwrap_err`.
- Classifier substrings and config.yml layout are assumptions until the 02-SPIKE run (marked `shortcut:` in code).

## Verification
- fmt + clippy `-D warnings`: clean. `check-file-sizes`: pass.
- `--lib gitlab`: 12 passed (includes 02-01 ca tests and the fake-glab tests). `--lib forge`: 7 passed (02-01 tests, previously unrun). Route test `gitlab_routes_...`: 1 passed. No phase-2 fixes were needed.
- Full suite not run (orchestrator).

## Deviations
None.

## Self-Check: PASSED
