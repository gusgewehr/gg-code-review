---
phase: 01-camada-de-provider-forge-sem-mudan-a-de-comportamento
verified: 2026-11-05T00:00:00Z
status: human_needed
score: 4/4 locally checkable must-haves verified (3 ROADMAP criteria need toolchain/Docker)
overrides_applied: 0
human_verification:
  - test: "cargo fmt --all --check; cargo clippy --workspace --all-targets --locked -- -D warnings; cargo test --workspace --locked (with OCR_TEST_DATABASE_URL)"
    expected: "All pass; forge::tests::github_urls_and_refspec_are_unchanged passes; no existing assertion edited"
    why_human: "cargo/rustc not installed locally and OCR_TEST_DATABASE_URL unset; compile-correctness checked by reading only"
  - test: "pnpm test"
    expected: "Passes (no frontend change in this phase)"
    why_human: "pnpm not installed locally"
  - test: "Rebuild server+runner images together; run docker/runner-smoke.py and the other Docker smokes"
    expected: "Pass unchanged. Executor fingerprint changes (review/mod.rs, dependencies.rs, frozen.rs, review_archive/mod.rs, indexer/mod.rs are hashed), so images must be rebuilt together"
    why_human: "docker not installed locally"
  - test: "Review and publish a real GitHub PR; compare comment bodies and URLs with a pre-refactor run"
    expected: "Byte-identical bodies and URLs"
    why_human: "Needs live GitHub PR and Docker stack"
---

# Phase 1: Forge provider layer, no behavior change - Verification Report

**Phase Goal:** O codigo fora de `backend/src/github/` passa a falar com uma interface de provider, e o fluxo GitHub continua exatamente igual.
**Status:** human_needed (no code gaps found; toolchain/Docker/real-PR items unverified)
**Re-verification:** No

## Automated checks environment

cargo, rustc, pnpm and docker are NOT installed locally and `OCR_TEST_DATABASE_URL` is unset. `cargo test`, `cargo clippy`, `cargo fmt`, `pnpm test`, Docker smokes and a real PR run were NOT executed. Everything below was checked by grep, diff and reading.

## Goal Achievement

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | No file outside `github/` and `forge/` references `github::` (ROADMAP SC4) | VERIFIED | Grep gate prints nothing. `use crate::{github` text in `indexer/tests.rs` is a string fixture, untouched, and does not match `github::` anyway |
| 2 | URLs and PR head refspec come from `Forge` methods, byte-identical | VERIFIED | Removed literals vs `forge/mod.rs` bodies compared in the diff (below) |
| 3 | Existing test assertions unchanged; new unit test pins strings (ROADMAP SC1, partial) | VERIFIED (static) | Diff of test files has only `github::` to `forge::` path renames (`usage.rs`, `comment_activation_tests.rs`, `status_comments_test_support.rs`, `server/tests.rs`). `forge::tests::github_urls_and_refspec_are_unchanged` added. Test execution itself is UNVERIFIED |
| 4 | GitHub behavior unchanged: forge items are github items via glob re-export | VERIFIED (static) | `pub(crate) use crate::github::*;`, no type/fn duplication. Pure line substitutions: equal add/remove numstat in all rename-only files |
| 5 | Docker smokes pass unchanged (SC2); real PR identical (SC3) | UNCERTAIN | Not runnable locally; see human_verification |

### Byte-identity of strings

| Site | Removed literal | New |
|------|-----------------|-----|
| `commit_url` | `https://github.com/{full_name}/commit/{sha}` (publication.rs:120, 1152, 1229) | `Forge::GitHub => format!("https://github.com/{full_name}/commit/{sha}")`, same text interpolated via `{}` |
| `review_url` | `https://github.com/{}/pull/{}#pullrequestreview-{id}` (publication.rs:701, review_http.rs:203, runtime.rs:334) | `https://github.com/{full_name}/pull/{number}#pullrequestreview-{review_id}`, same |
| `pr_head_ref` | `refs/pull/{number}/head` (review/git.rs:22, summary.rs:249) | `refs/pull/{number}/head`; callers still append `:refs/ocr/pr-{number}` and `:refs/ocr/summary-{number}` |

The warning/"Reviewed" bodies keep the emoji and `\n\n` unchanged. Hard-coded `github.com` retained (not `gh_host()`), matching the old behavior.

### Compile-correctness by reading

- `forge::current` and `Forge` do not collide with any github item. The only `tests` in github is a private `mod tests;`, which a glob does not import.
- `forge` is imported in `server/mod.rs` and `review/mod.rs` via `use crate::{forge, ...}`. Submodules use `use super::*;` as `github` did before. No local identifier named `forge` shadows it.
- Argument types match: `number: i64` (review_http.rs tuple from `pr_review_coords`, `post.number`, `inputs.number`), `full_name` is `String` at `&full_name` sites and `&str` in publication.rs helpers, `review_id: i64` (unwrapped from `Option`/match arm). `review_id.map(|id| ...)` yields `i64`.
- `mod forge;` is registered in `lib.rs` (after `pub mod db;`, a harmless deviation from the plan).
- `#[allow(unused_imports)]` covers the unused-glob lint. `Forge` methods are all used, so no dead-code warning is expected.
- No added line over 121 chars except pre-existing long lines that only changed by the rename.

### Other checks

- `node scripts/check-file-sizes.mjs`: passed (289 files).
- `git diff` on `backend/src/github/` and `indexer/tests.rs`: untouched.

### Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|-------------|-------------|--------|----------|
| FORGE-02 | 01-01 | SATISFIED in code, pending CI/Docker confirmation | Seam exists and every external call routes through it. REQUIREMENTS.md already marks it `[x]` but the traceability table (line 97) still says "Pendente"; update it after CI confirms. No orphaned Phase 1 requirements |

### Anti-Patterns

None. No TBD/FIXME/XXX introduced. The `shortcut:` comment on `current()` is an intentional, documented limit (Phase 2 selects the provider).

## Gaps Summary

No code gaps. Remaining risk is only unexecuted compile/test/Docker/real-PR verification. Executor fingerprint changes by design, so server and runner images must be rebuilt together.

_Verifier: Claude (gsd-verifier)_

## Post-execution automated checks (2026-11-05, WSL Docker, rust:1.98.1-bookworm)

- `cargo fmt --all --check`: pass
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass
- `cargo test --workspace --locked` against postgres:17: 649 passed, 1 failed (`review::evidence::tests::omits_only_the_known_presentation_section`) only because the Windows checkout has CRLF in `backend/src/assets/review-skill.md`; with LF-normalized tree (as on Linux CI) it passes. Net: all pass.
- `pnpm test` (Node 25 on Windows): 20 failures in `localStorage.clear()` are environment baseline (jsdom vs Node 25 native localStorage); phase 1 touched no frontend.
- Still manual: Docker smokes and real-PR byte comparison (01-HUMAN-UAT.md). Deferred by autonomous run.
