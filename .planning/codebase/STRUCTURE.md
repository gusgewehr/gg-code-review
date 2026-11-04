---
last_mapped_commit: 1a47d4f35f3f87acdd5dd4e1767477628dd27a25
---
# Codebase Structure

**Analysis Date:** 2026-11-04

## Directory Layout

```
opencodereview/
├── src/                    # React frontend
│   ├── pages/              # Onboarding screens
│   ├── components/         # Shared UI (DesignSystem, BrandLoading, Toast, ...)
│   ├── services/           # setupApi*.ts HTTP client modules
│   ├── platform/           # Main app: components/, hooks/, lib/, pages/
│   ├── styles/             # globals.css (theme tokens), components.css
│   └── types/, lib/
├── backend/                # Rust crate `opencodereview-server` (workspace member)
│   └── src/
│       ├── server/         # HTTP API, routes, workers, publication, auth flows
│       ├── review/         # Review orchestration, validation, integration/, tests/
│       ├── review_archive/ # Frozen evidence, retention, purge
│       ├── sandbox/        # runner, egress, read_commands, tool guard
│       ├── providers/      # claude.rs, codex.rs, engines.rs
│       ├── accounts/       # AI credentials, usage, sandbox_sync
│       ├── github/         # gh CLI wrappers
│       ├── indexer/        # manifests, imports, symbols
│       ├── db/             # per-aggregate queries, pg/ migrations
│       ├── bench/          # ocr-bench support
│       ├── assets/         # review-skill.md
│       └── main_server.rs, main_sandbox.rs, main_bench.rs, lib.rs, state.rs, local.rs, path_kinds.rs
├── docker/                 # Dockerfiles, nginx conf, smoke/diagnostic scripts
├── bench/                  # Public benchmark adapters (Python), manifests, report.py
├── scripts/                # Node scripts: changelog, file sizes, dev web, release, og, site
├── changelog.d/            # Release notes per branch
├── site/, assets/          # Marketing site; static brand/onboarding assets
├── .agents/skills/         # git, comments agent workflows
├── docker-compose*.yml     # Runtime (base, images, scale)
└── package.json, biome.json, knip.json, tsconfig.json, vite.config.ts, Cargo.toml, deny.toml
```

## Directory Purposes

**`src/platform/pages/`:**
- Purpose: top-level routes (`ReviewsPage`, `IndexingPage`, `SkillsPage`, `SettingsPage`, `UsagePage`), with subfolders `reviews/`, `settings/`, `usage/` for page parts.

**`backend/src/server/`:**
- Purpose: one file per API area; re-exported through `server/mod.rs` with `pub(crate) use`.
- Key files: `routes.rs`, `http.rs`, `review_jobs.rs`, `polling.rs`, `publication.rs`, `runtime.rs`.

**`backend/src/db/`:**
- Purpose: `impl Database` methods split by aggregate (`review_runs.rs`, `findings.rs`, `settings.rs`, ...). Migrations in `db/pg/NNNN_name.sql`.

**`backend/src/review/`:**
- Purpose: pipeline pieces; `integration/` (final batches), `tests/` (cross-module tests).

## Key File Locations

**Entry Points:**
- `backend/src/main_server.rs`, `backend/src/main_sandbox.rs`, `backend/src/main_bench.rs`
- `src/main.tsx`, `src/router.tsx`

**Configuration:**
- `backend/Cargo.toml`, `Cargo.toml`, `package.json`, `biome.json`, `vite.config.ts`, `docker-compose.yml`, `docker/web-nginx.conf`

**Core Logic:**
- `backend/src/review/execution.rs`, `backend/src/review/chunks.rs`, `backend/src/sandbox/runner.rs`, `backend/src/providers/claude.rs`, `backend/src/providers/engines.rs`

**Testing:**
- Rust: sibling `*_tests.rs` files and `#[cfg(test)]` modules in the same directory (e.g. `review/findings_tests.rs`, `server/tests.rs`)
- Frontend: co-located `*.test.ts(x)` (Vitest, jsdom)
- Docker: `docker/runner-smoke.py`, `docker/test_diagnostics.py`, `docker/test-web-proxy.py`

## Naming Conventions

**Files:**
- React components: PascalCase `.tsx` (`ReviewFlow.tsx`); hooks `useX.ts`; libs camelCase (`reviewFlow.ts`).
- Tests: `Name.test.tsx` beside source; variants `ReviewsPage.decisions.test.tsx`.
- Rust: snake_case modules; tests `<module>_tests.rs`.
- Migrations: `NNNN_snake_name.sql`, appended in order.

**Directories:** lowercase (`platform/pages/reviews`), snake_case in Rust (`review_archive`).

## Where to Add New Code

**New API endpoint:** handler in the matching `backend/src/server/<area>.rs`, route entry in `server/routes.rs`; query in `backend/src/db/<aggregate>.rs`; client call in `src/services/setupApi<Area>.ts`.

**New DB change:** new SQL file at end of `backend/src/db/pg/` and register in `migrate.rs`; never edit applied ones.

**New review stage logic:** `backend/src/review/`; keep under 2,500 lines per file. Changes to executor sources alter the fingerprint (`sandbox/mod.rs`).

**New model:** only in `backend/src/providers/engines.rs`. CLI args only in `providers/claude.rs` / `codex.rs`.

**New UI page/component:** page in `src/platform/pages/`, component in `src/platform/components/` (or `src/components/` if shared), reuse `src/components/DesignSystem.tsx`; Tailwind tokens from `src/styles/globals.css`; add route in `src/router.tsx`; test beside it.

**Utilities:** frontend `src/platform/lib/`; Rust shared helpers `backend/src/path_kinds.rs` or the relevant module.

**Release note:** `changelog.d/<branch>.md` for user-visible changes.

## Special Directories

**`backend/src/db/pg/`:** SQL migrations; generated: no; committed: yes.
**`backend/src/assets/`:** built-in review skill; committed.
**`.planning/`:** planning docs; `node_modules/`, `target/`, `dist/` are generated and not committed.
**`db/schema.sql`, `gemini_accounts.sql`, `more_providers.sql`:** legacy SQLite import only.

---

*Structure analysis: 2026-11-04*
