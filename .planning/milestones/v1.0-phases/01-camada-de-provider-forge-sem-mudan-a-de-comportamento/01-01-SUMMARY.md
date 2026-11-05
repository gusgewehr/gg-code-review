---
phase: 01-camada-de-provider-forge-sem-mudan-a-de-comportamento
plan: 01
subsystem: backend
tags: [forge, refactor, github]
requires: []
provides: [forge seam (backend/src/forge), Forge::commit_url/review_url/pr_head_ref]
affects: [server, review, review_archive, indexer, db]
tech-stack:
  added: []
  patterns: [glob re-export seam, enum dispatch via match]
key-files:
  created: [backend/src/forge/mod.rs]
  modified: [backend/src/lib.rs, backend/src/server/*, backend/src/review/*, backend/src/review_archive/*, backend/src/indexer/mod.rs, backend/src/db/review_requests.rs]
decisions:
  - "forge::current() free fn returns Forge::GitHub (shortcut; Phase 2 reads OCR_FORGE) instead of a ServerConfig field"
metrics:
  completed: 2026-11-05
requirements: [FORGE-02]
---

# Phase 1 Plan 01: Forge seam Summary

`backend/src/forge/` re-exports `crate::github::*` and adds `enum Forge { GitHub }` with `commit_url`, `review_url` and `pr_head_ref` (byte-identical to the old literals, pinned by a unit test); all `github::` call sites outside `github/` now use `forge::`.

## Commits
- 83e3580: refactor(forge): route github calls and URLs through a forge seam (Tasks 1 and 2 in one commit)

## Deviations from Plan
- Tasks 1 and 2 were committed together (hunks overlap in the same files). No functional deviation.
- `mod forge;` placed after `pub mod db;` (alphabetical, for rustfmt module ordering) rather than after `mod accounts;`.

## Verification
- Grep gate: no `github::` outside `github/` and `forge/` except the string fixture in `indexer/tests.rs` (untouched); no `https://github.com/{` left in `server/`; `node scripts/check-file-sizes.mjs` passes.
- NOT run locally (no toolchain): `cargo fmt/clippy/test/check`, `pnpm test`, Docker smokes. Must be confirmed in CI. Code was hand-formatted for rustfmt (max_width 120); import ordering after the rename (e.g. `use crate::forge::...` in sorted groups) may need `cargo fmt`.
- Executor fingerprint (`sandbox::engine_version`) changes because hashed files were edited (review/mod.rs, dependencies.rs, frozen.rs, review_archive/mod.rs, indexer/mod.rs): rebuild server and runner images together.

## Known Stubs
None.

## Self-Check: PASSED
forge/mod.rs exists; commit 83e3580 exists.
