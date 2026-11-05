---
phase: 02-conex-o-com-o-gitlab-self-hosted
plan: 05
subsystem: infra
tags: [docker, glab, ci]
requires: []
provides:
  - valid glab install RUN in the server image (AUTH-05)
  - static Dockerfile continuation check in CI
affects: [docker/Dockerfile, docker/test-gitlab-isolation.py]
key-files:
  modified:
    - docker/Dockerfile
    - docker/test-gitlab-isolation.py
    - .planning/REQUIREMENTS.md
decisions: []
metrics:
  tasks: 2
  files: 3
completed: 2026-11-05
---

# Phase 2 Plan 05: Dockerfile glab RUN fix Summary

Server Dockerfile glab install restored to a valid multi-line RUN (pinned 1.128.0, per-arch sha256 check), with a static continuation check added to the existing CI isolation script.

## Tasks

1. Rewrote the glab RUN in Codex style; added `check_dockerfile_continuations` (RED confirmed: failed at `docker/Dockerfile:74`; GREEN: all five ok lines). Commit 0c29d1d.
2. REQUIREMENTS.md: AUTH-01/02/03/05 checked; AUTH-01..05 and FORGE-01 marked `Implementado`. Commit 6a2511a.

## Deviations from Plan

None. The working tree has CRLF via core.autocrlf, but the index stays LF (`i/lf`), so the commit is LF.

## Verification

- `python docker/test-gitlab-isolation.py` passes.
- `docker build` not run (orchestrator will run it; memory constrained).
- Dockerfile.runner untouched.

## Self-Check: PASSED
