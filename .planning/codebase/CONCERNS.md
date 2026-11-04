---
last_mapped_commit: 1a47d4f35f3f87acdd5dd4e1767477628dd27a25
---
# Codebase Concerns

**Analysis Date:** 2026-11-04

Scope: full repo. Findings come from a static scan (size, markers, unsafe, unwrap/expect, lint allows) plus `AGENTS.md`. No tests or builds were run. Nothing here is a confirmed bug; the project has no TODO/FIXME/HACK/XXX markers in `src/`, `backend/src/`, `docker/` or `bench/`.

## Tech Debt

**Rust files near the 2,500-line limit (`pnpm check:file-size`):**
- Issue: several files sit close to the cap, so the next feature in them forces a split.
- Files: `backend/src/review/chunks.rs` (2372, non-test), `backend/src/review/evidence.rs` (1566), `backend/src/server/publication.rs` (1444), `backend/src/sandbox/runner.rs` (1324), `backend/src/review/execution.rs` (1198), `backend/src/review/dependencies.rs` (1191).
- Test files also near the cap: `backend/src/db/mod_tests.rs` (2420), `backend/src/review/tests/usage.rs` (2357), `backend/src/review/integration/tests.rs` (2079).
- Impact: CI file-size check fails on any growth in `chunks.rs`, `mod_tests.rs` and `usage.rs`.
- Fix approach: split `chunks.rs` by stage (chunk planning, validation, repair) before adding review logic. Split test files by theme.

**Frontend files near the 800-line limit:**
- Files: `src/platform/pages/ReviewsPage.tsx` (782), `src/platform/lib/reviewFlow.ts` (779).
- Impact: any addition to review UI or flow logic breaks `pnpm check:file-size`.
- Fix approach: extract sub-components and flow helpers into sibling modules.

**Wide function signatures:**
- Issue: `#[allow(clippy::too_many_arguments)]` at `backend/src/review/chunks.rs:693`, `backend/src/review/lens.rs:81`, `backend/src/indexer/mod.rs:378`; `#[allow(clippy::large_enum_variant)]` at `backend/src/review/chunks.rs:820`.
- Impact: parameter-order mistakes; the review pipeline is hard to extend.
- Fix approach: introduce a context struct for stage parameters.

**Legacy SQLite path:**
- Issue: `backend/src/db/schema.sql` and its SQLite migrations are kept only for `ocr-server import-sqlite`.
- Impact: two schema definitions exist; the import path is rarely exercised.
- Fix approach: drop it when the import window closes.

**Panics in non-test Rust:**
- Issue: about 134 `.unwrap()`/`.expect(` occurrences under `backend/src` (count includes inline test modules, so the production share is lower).
- Impact: a panic in the server or a job thread aborts the job instead of returning an actionable error.
- Fix approach: audit with `rg "unwrap\(\)|expect\(" backend/src --glob '!*tests*'`; convert the ones on external input (Git, GitHub, runner IPC, model reports).

## Known Bugs

None confirmed from static analysis. The `AGENTS.md` review invariants (quote line drift up to 3 lines, unique path completion, PR_DIFF.patch line remapping) are heuristics that can mis-anchor in edge cases. They are the likeliest source of wrong-line findings.

## Security Considerations

**`unsafe` / libc usage:**
- Risk: memory and FD handling errors.
- Files: `backend/src/review/git.rs:10` (process-group `kill(-pgid)`), `backend/src/review_archive/mod.rs:967-995` (raw `File::from_raw_fd`), `backend/src/server/runtime.rs:31` (`gethostname`), `backend/src/db/pg/test_support.rs:274,301` (test only).
- Current mitigation: small, localized blocks; `pid > 1` guard in `kill_tree`.
- Recommendations: keep each block commented with its invariant; prefer `nix`/`rustix` wrappers if one is already a dependency.

**Sandbox boundary:**
- Risk: model output executing tools or reaching the network.
- Files: `backend/src/providers/claude.rs` (CLI args, tool policy), `backend/src/sandbox/egress.rs` (host allow-list), `backend/src/sandbox/runner.rs`, tool-guard hook.
- Current mitigation: `PreToolUse` guard on every call, denied tools (`Agent`, `Task`, `WebFetch`, `WebSearch`, `mcp__*`), egress allow-list, runner gets only IPC socket and a read-only sign-in projection.
- Recommendations: any change to these files needs `python3 -u docker/runner-smoke.py`. Keep one runner per container, since the survivor check covers every process of the model UID.

**Credential handling:**
- Risk: leaks of tokens or DB URLs to logs or the browser.
- Files: `backend/src/accounts/`, redaction code (see commit 9225565, "redact credentials in linear time").
- Current mitigation: redaction on output; `GH_TOKEN` is a development fallback only.
- Recommendations: add a redaction test for each new provider or log path.

**`dangerouslySetInnerHTML`:**
- Risk: XSS if the source becomes dynamic.
- Files: `src/components/BrandLoading.tsx:42` (static `artwork` string).
- Current mitigation: the input is a bundled constant.
- Recommendations: keep it static; do not feed it user or API data.

**Pinned provider CLIs:**
- Risk: pinned Claude and Codex versions drift from provider behavior or go stale.
- Files: `docker/` Dockerfiles, `.github/workflows/provider-cli-canary.yml`.
- Current mitigation: checksum-verified pins and a canary workflow.
- Recommendations: bump pins deliberately; run the canary first.

## Performance Bottlenecks

**Large-repository evidence freezing:**
- Problem: repositories over the archive budgets are truncated (changed, related, test and convention files first, then path order). Files past the budget cannot be cited.
- Files: `backend/src/review_archive/mod.rs`, `backend/src/review/evidence.rs`.
- Cause: fixed byte and manifest budgets.
- Improvement path: improve prioritization of related files; surface truncation in the UI.

**Runner fan-out:**
- Problem: 20 runner replicas by default (`OCR_REVIEW_RUNNERS`) consume memory on small hosts.
- Files: `docker-compose.yml`.
- Improvement path: document sizing; lower the default for local use.

## Fragile Areas

**Review validation and repair pipeline:**
- Files: `backend/src/review/chunks.rs`, `backend/src/review/execution.rs`, `backend/src/review/lens.rs`, `backend/src/review/integration/`.
- Why fragile: many interacting invariants (drop vs fail, repair limited to one extra execution, lens merge revalidation, deadline gating, `unchanged` candidates, unconfirmed marking).
- Safe modification: change one rule at a time and extend the matching tests in `backend/src/review/tests/` and `backend/src/review/integration/tests.rs`. Never auto-rerun inference after a startup failure.
- Test coverage: extensive, but the test files are themselves near the size cap.

**Publication:**
- Files: `backend/src/server/publication.rs`.
- Why fragile: must stay idempotent and bound to the exact validated selection, and keep the `<!-- ocr-... -->` markers.
- Safe modification: keep `lock_pull_request` before state-dependent writes; do not hold a `Database` across Git, GitHub or model calls.

**Multi-instance background work:**
- Files: `backend/src/db/`, `backend/src/server/`.
- Why fragile: work that must not run twice must use `Db::run_exclusive`. Missing it causes duplicate jobs.

**Git/GitHub forge coupling:**
- Files: `backend/src/github/`, `backend/src/review/git.rs` (uses `refs/pull/{n}/head` and `github.com` transport).
- Why fragile: the fetch refspec is GitHub-specific. The maintainer's memory notes a planned GitLab provider layer (`glab`); the GitHub assumptions here will need abstraction first, and `AGENTS.md` currently limits GitHub access to the `gh` CLI.

## Scaling Limits

**Database migrations:**
- Current capacity: append-only SQL in `backend/src/db/pg/`, listed in `migrate.rs`.
- Limit: a database newer than the app fails explicitly (no downgrade path).
- Scaling path: roll servers and runners forward together (executor fingerprint must match).

## Dependencies at Risk

Not assessed beyond CI. `cargo deny check` and `.github/workflows/audit.yml` cover advisories, licenses and sources. Run `pnpm knip` to find unused dependencies.

## Missing Critical Features

Not applicable. `AGENTS.md` fixes scope (reviews, indexing, skills) and intentionally excludes cost estimates, extra providers and managed GitHub tokens.

## Test Coverage Gaps

**Paid-inference paths:**
- What's not tested: real model output (needs maintainer authorization); CI uses fake inference (`docker/runner-smoke.py`).
- Files: `backend/src/providers/`, `bench/`.
- Risk: provider CLI output changes surface only in the canary or in benchmarks.
- Priority: Medium.

**DB-dependent Rust tests:**
- What's not tested: anything is skipped silently if `OCR_TEST_DATABASE_URL` is unset locally.
- Files: `backend/src/db/pg/test_support.rs`.
- Risk: local runs pass without exercising the database ("zero matching tests is not a pass").
- Priority: Low.

**Frontend:**
- What's not tested: visual and Docker-integration behavior; Vitest with jsdom covers interactions only.
- Files: `src/platform/`, `src/pages/`.
- Priority: Low.

---

*Concerns audit: 2026-11-04*
