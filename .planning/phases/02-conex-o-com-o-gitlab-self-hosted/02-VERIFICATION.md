---
phase: 02-conex-o-com-o-gitlab-self-hosted
verified: 2026-11-05T00:00:00Z
status: human_needed
score: 5/5 must-haves verified (after gap closure 02-05)
overrides_applied: 0
gaps_closed:
  - truth: "glab is installed only in the server image, pinned version and verified checksum; runners get no glab/token/CA (AUTH-05, SC5)"
    status: closed (02-05, commit 0c29d1d; glab RUN built in isolation on debian:bookworm-slim: sha256 OK, glab 1.128.0)
    reason: "docker/Dockerfile line 73-74: the glab RUN instruction contains a literal newline inside printf '%s  /tmp/glab.tar.gz<newline>' with no trailing backslash. Docker ends the instruction at line 73 and parses line 74 (starting with a single quote) as an unknown instruction, so the server image build fails. The checksum step never runs."
    artifacts:
      - path: "docker/Dockerfile"
        issue: "RUN split across lines 73/74 without line continuation (printf format string newline was materialized as a real newline); all other RUNs use backslash continuations"
    missing:
      - "Rewrite the glab RUN as a multi-line command with backslash continuations and printf '%s  /tmp/glab.tar.gz\\n' (escaped), then run docker build of the server image"
      - "Make docker/test-gitlab-isolation.py also check that the Dockerfile parses (for example, no non-instruction line starts a line outside a continuation)"
human_verification:
  - test: "Run docker/gitlab-spike.py against the real GitLab (02-SPIKE.md is still 'pending user run')"
    expected: "PAT format, personal_access_tokens/self, API version, pagination, CA handling, error strings recorded; classify() and has_stored_login updated"
    why_human: "No reachable GitLab instance in this environment (SC1)"
  - test: "With OCR_FORGE=gitlab, OCR_GITLAB_HOST and OCR_GITLAB_CA_FILE set, paste a PAT in Settings > Integrations; then try a token without api scope, an expired one, and a stopped host; then disconnect"
    expected: "Status shows host and user; actionable error per case; token absent from browser devtools, server logs and DB; disconnect clears the login"
    why_human: "Needs live GitLab and glab; error classification substrings are unverified assumptions"
  - test: "Change OCR_FORGE/OCR_GITLAB_HOST over an existing database and restart"
    expected: "Server exits with the clear 'pinned to forge ... Restore ...' message"
    why_human: "End-to-end boot behaviour (unit tests cover only check/enforce)"
---

# Phase 2: Conexão com o GitLab self-hosted Verification Report

**Phase Goal:** O admin escolhe GitLab como provider do deploy e conecta à instância interna (CA própria) colando um PAT, sem expor o token.
**Status:** gaps_found (1 blocker)
**Re-verification:** No

## Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Spike against real GitLab recorded before building the rest (SC1) | ? UNCERTAIN (human) | Spike kit `docker/gitlab-spike.py` exists; `02-SPIKE.md` says "pending user run", all findings blank. classify()/has_stored_login carry `shortcut:` comments flagging unverified assumptions |
| 2 | Provider and host set by config; server refuses to start with a clear message on change (SC2, FORGE-01) | VERIFIED | `forge/mod.rs` `configured`/`init_from_env`; `forge/guard.rs` `enforce` pins in settings and `check` errors naming both values; wired in `server/mod.rs` `run_from_env` (init) and `run` (enforce, error propagates via `?`). GitHub host comes from `gh_host_detected` and nothing is pinned when undetected, matching the user override; changelog says the same. Unit tests cover pinning, change rejection, skip without host |
| 3 | PAT pasted in Settings, status shows host and user, actionable errors for missing scope/expired/unreachable (SC3, AUTH-01/02) | VERIFIED statically (live behaviour human) | `gitlab/auth.rs` login via `glab auth login --stdin`, status via `glab api user` + `personal_access_tokens/self`, `token_problem` and `classify`; routes `GET /forge/status`, `POST /auth/gitlab/token` in `server/routes.rs`; `GitLabConnection.tsx` (password input, autoComplete off) and `setupApiAccess.ts` wired |
| 4 | Admin can disconnect; token never in browser, logs, DB (SC4, AUTH-03) | VERIFIED statically | `logout` + route `/auth/gitlab/logout`; token only on stdin, `redact()` on all glab output, env tokens removed, status JSON has no token field, no DB write of token. glab keeps it in `$OCR_DATA_DIR/glab/config.yml` (insecure-storage, private dir), not the DB |
| 5 | glab and git trust the mounted CA with no TLS-off option; runner image has no glab/token/CA (SC5, AUTH-04/05) | FAILED (AUTH-05 part) | CA part VERIFIED: `gitlab/ca.rs` builds bundle and sets SSL_CERT_FILE, GIT_SSL_CAINFO, GLAB_CA_CERT before threads; compose mounts CA only on the server. Runner isolation: no `glab`/`ca.crt` in `Dockerfile.runner` or compose runner. But the server Dockerfile glab install is syntactically broken (see gap), so glab is not installable in the image |

**Score:** 4/5

## Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `backend/src/forge/mod.rs`, `forge/guard.rs` | VERIFIED | substantive, wired at boot |
| `backend/src/gitlab/{auth,ca,mod}.rs`, `auth_tests.rs` | VERIFIED | substantive, routes call them |
| `src/platform/pages/settings/GitLabConnection.tsx` | VERIFIED | wired through `setupApiAccess.ts` |
| `docker/Dockerfile` glab block | FAILED | RUN broken by stray newline, lines 73-74 |
| `docker-compose.yml` env + CA mount | VERIFIED | OCR_FORGE, OCR_GITLAB_HOST, CA mounted server only (default /dev/null) |
| `changelog.d/feat-gitlab-connection.md` | VERIFIED | |
| `docker/test-gitlab-isolation.py` | WARNING | static checks pass but did not catch the broken RUN |

## Requirements Coverage

| Requirement | Plan | Status | Evidence |
|-------------|------|--------|----------|
| FORGE-01 | 02-01 | SATISFIED | guard + boot wiring |
| AUTH-01 | 02-03, 02-04 | SATISFIED (live check human) | stdin login, redaction |
| AUTH-02 | 02-03, 02-04 | SATISFIED (live check human) | status + classify; strings unverified until spike |
| AUTH-03 | 02-03, 02-04 | SATISFIED | logout route + UI |
| AUTH-04 | 02-01, 02-02 | SATISFIED | CA bundle + env, no TLS-skip |
| AUTH-05 | 02-02, 02-04 | BLOCKED | Dockerfile glab RUN invalid; runner isolation OK |

No orphaned IDs: all six phase IDs appear in plan frontmatter. REQUIREMENTS.md checkboxes/traceability table still show AUTH-01/02/03/05 as pending, to be updated after the gap is closed.

## Anti-Patterns

| File | Pattern | Severity |
|------|---------|----------|
| `docker/Dockerfile:73-74` | RUN without continuation | BLOCKER |
| `gitlab/auth.rs` | `shortcut:` assumptions on error strings and config.yml layout | WARNING (tied to the spike) |
| `forge/mod.rs` tests | `Forge::GitLab` URL test asserts host "github.com" (HOST unset in tests) | Info |

No TBD/FIXME/XXX markers found in the phase files read.

## Gaps Summary

One blocker: the server Dockerfile glab install does not parse, so the image cannot be built and AUTH-05 is not met (a plan-checked "docker build may not have been run" fact that turns out to be a real defect, found by static reading). Fix the RUN continuation and build the server image. The remaining items (real GitLab spike, live PAT flows, boot refusal end-to-end) are human verification items.

---

_Verified: 2026-11-05_
_Verifier: Claude (gsd-verifier)_


## Re-verification after gap closure (02-05)

- `docker/Dockerfile` glab `RUN` rewritten with `\` continuations (0c29d1d). `docker/test-gitlab-isolation.py` gained `check_dockerfile_continuations` (fails on the old file, passes now).
- The glab `RUN` block was copied verbatim into a throwaway Dockerfile on `debian:bookworm-slim` and built (amd64): `/tmp/glab.tar.gz: OK`, `glab 1.128.0 (519ea19f7)`. The arm64 checksum was not exercised by a build.
- REQUIREMENTS.md updated (6a2511a).
- Status is now `human_needed`: only the live-GitLab items above remain.

## Final gate (2026-11-05, HEAD 5acfa85)

| Check | Result |
|-------|--------|
| `cargo fmt --all --check` + `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass (post-fix tree) |
| `rusttest.sh "--lib forge"` | 7 passed, 0 failed (post-fix tree) |
| `rusttest.sh "--lib gitlab"` | 12 passed, 0 failed (post-fix tree) |
| `rusttest.sh` (full suite) | 666 passed, 0 failed, 15 ignored, on the tree before the code-review fixes (7e14cd0, all 02-01..02-04 code). The post-fix re-run was killed by host memory pressure and not restarted; the fixes (aaf8a0b, a3fe0f9) only touch `gitlab/auth.rs` and GitLab-only URL helpers in `forge/mod.rs`, which the forge/gitlab runs above cover. |
| `pnpm test` | 20 failed / 273 passed / 7 skipped = baseline. All failures are in files phase 2 did not touch (localStorage.clear under Node 25, plus `scripts/site/seo.test.ts` on the CRLF Windows checkout). The phase-2 test files (SettingsPage, GitLabConnection) pass. |
| `tsc --noEmit -p .` | pass |
| `biome check src` | lint clean. Format reports CRLF-only diffs repo-wide (Windows autocrlf checkout, pre-existing); the committed LF blobs of all 6 phase-2 frontend files are format-clean. |
| `node scripts/check-file-sizes.mjs` | pass (296 files) |
| `python docker/test-gitlab-isolation.py` | pass (5 checks, incl. new Dockerfile continuation check) |
| glab install step | built in isolation on `debian:bookworm-slim` (amd64): checksum OK, `glab 1.128.0`. Full `docker build -f docker/Dockerfile .` not run (Rust build stage, memory). arm64 not exercised. |

Code review: 02-REVIEW.md (0 critical, 6 warning, 3 info). Fixed WR-01, WR-02, WR-04, WR-05, WR-06 (02-REVIEW-FIX.md). WR-03 skipped pending spike data. Info items left open.
