---
phase: 02-conex-o-com-o-gitlab-self-hosted
plan: 02
subsystem: infra
tags: [docker, glab, gitlab, ci, spike]
requires: []
provides:
  - glab 1.128.0 in the server image, checksum-verified per arch
  - compose OCR_FORGE/OCR_GITLAB_HOST env and read-only CA mount (server only)
  - CI static isolation and no-TLS-skip check
  - spike kit for the real GitLab
affects: [02-01, 02-03]
key-files:
  created:
    - docker/test-gitlab-isolation.py
    - docker/gitlab-spike.py
    - changelog.d/feat-gitlab-connection.md
    - .planning/phases/02-conex-o-com-o-gitlab-self-hosted/02-SPIKE.md
  modified:
    - docker/Dockerfile
    - docker-compose.yml
    - .github/workflows/ci.yml
metrics:
  completed: 2026-11-05
---

# Phase 2 Plan 02: Deploy side of the GitLab connection Summary

glab 1.128.0 pinned with per-arch sha256 in the server image only, compose wires forge env and the CA mount to the server only, and CI statically enforces runner isolation and no TLS-skip options.

## Tasks

1. Dockerfile glab block, compose env/CA mount, isolation check, CI step, changelog (commit in git log: `feat(02-02): pin glab in server image...`).
2. Spike kit: `docker/gitlab-spike.py` and `02-SPIKE.md` (status pending user run).

## Deviations from Plan

- **Changelog (user override):** the `Changed` note says GitHub deploys pin `github` plus the host already detected from OCR_GH_HOST/GH_HOST or the gh login (nothing pinned if none is detected); it does not tell GHE users to set OCR_GH_HOST.
- **[Rule 1 - Bug]** The runner-image check matched `ocr-ca` inside `ocr-cargo-registry`; narrowed to `/run/ocr-ca`.

## Verification

- `python -B docker/test-gitlab-isolation.py`, `pnpm changelog:check` and `docker compose config --quiet` pass.
- glab sha256 values confirmed against the official checksums.txt for v1.128.0 (curl).
- Not done: `docker build` (skipped per instructions), so `glab --version` in-image and the tarball layout are unverified. Spike not run (no instance reachable).

## Self-Check: PASSED
