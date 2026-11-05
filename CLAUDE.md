<!-- GSD:project-start source:PROJECT.md -->
## Project

**OpenCodeReview — GitLab self-hosted**

Fork do OpenCodeReview (BridgLabs, v0.2.0) que adiciona suporte a GitLab self-hosted. Um deploy configurado para GitLab descobre merge requests por polling, revisa com Claude Code ou Codex no sandbox existente e publica os findings como discussões inline no diff do MR, com a mesma experiência que o produto já oferece para pull requests do GitHub. É para times que hospedam o código num GitLab próprio, começando pelo time do autor.

**Core Value:** Um MR aberto numa instância GitLab self-hosted (com CA interna) é revisado e recebe findings inline automaticamente, sem quebrar nada do fluxo GitHub existente.

### Constraints

- **Runtime**: só Docker Compose — regra do `AGENTS.md`; `glab` entra na imagem do server com versão fixa e checksum verificado, sem instalação em runtime
- **Segurança**: nenhum token GitLab em código, logs, banco ou browser; runners nunca recebem credenciais de hospedagem
- **Arquitetura**: preservar invariantes de revisão (escopo com refs vivos, fetch falho falha a preparação, evidência congelada, publicação idempotente)
- **Compatibilidade**: zero regressão no fluxo GitHub; migrations novas só no fim da lista, nunca editar as aplicadas
- **Tamanho de arquivo**: Rust ≤ 2.500 linhas, frontend ≤ 800 linhas (`pnpm check:file-size`)
- **Estilo**: Conventional Commits, sem atribuição de AI em commits/PRs, release notes em `changelog.d/`
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust (edition 2021, toolchain 1.98.1) - backend in `backend/src/` (crate `opencodereview-server`, workspace root `Cargo.toml`)
- TypeScript ~5.9 - React frontend in `src/`, `src/platform/`
- Python 3 - Docker smoke/diagnostic scripts in `docker/*.py`, benchmark tooling `bench/report.py`
- Node (ESM `.mjs`) - repo scripts in `scripts/` (file-size check, changelog, site build, OG images)
- SQL - Postgres migrations in `backend/src/db/pg/*.sql`
- Shell - `docker/bench-entrypoint.sh`, `docker/web-entrypoint.sh`, `docker/postgres/init-roles.sh`
## Runtime
- Docker Compose is the only product runtime (`docker-compose.yml`, `docker-compose.images.yml`, `docker-compose.scale.yml`)
- Server/runner images: `debian:bookworm-slim` over `rust:1.98.1-bookworm` build stage (`docker/Dockerfile`, `docker/Dockerfile.runner`)
- Web image: `node:22-bookworm-slim` build, `nginx:1.30-alpine` serve (`docker/Dockerfile.web`)
- Database: `postgres:17` pinned by digest
- Sandbox uses `bubblewrap` in the image
- pnpm 10.15.0 (`packageManager` in `package.json`); lockfile `pnpm-lock.yaml` present
- Cargo, lockfile `Cargo.lock` present (`--locked` everywhere); `deny.toml` for cargo-deny
## Frameworks
- React 19.3 + react-dom - UI
- TanStack Router 1.170 / React Query 5 - routing and server state
- Tailwind CSS 4.3 (`@tailwindcss/vite`), theme tokens in `src/styles/globals.css`
- TipTap 3.32 (editor), `@xyflow/react` 12.12, `react-force-graph-2d`, `recharts`, `react-markdown` + `remark-gfm` + `rehype-raw`, `nuqs`, `lucide-react`
- Backend: no web framework; custom HTTP server in `backend/src/server/http.rs`, `routes.rs`, with `reqwest` (blocking) for outbound HTTP and thread pools (`scheduled-thread-pool`, `r2d2`)
- Vitest 4.1 with jsdom 26 (`vite.config.ts`)
- Rust `cargo test` (needs `OCR_TEST_DATABASE_URL`)
- Playwright 1.64 (devDependency)
- Vite 7.3 (`vite.config.ts`, dev port 1420/1421), `tsc`
- Biome 2.6.4 lint/format (`biome.json`: 2 spaces, width 120, double quotes)
- Knip (`knip.json`), clippy `-D warnings`, rustfmt
## Key Dependencies
- `postgres` 0.19 + `r2d2_postgres` + `tokio-postgres-rustls` - Postgres access with TLS
- `rusqlite` 0.32 (bundled) - only for `ocr-server import-sqlite`
- `reqwest` 0.13 (blocking, form, json), `rustls` 0.23 + `rustls-native-certs`
- `serde`/`serde_json`, `sha2`, `base64`, `libc`, `pulldown-cmark`, `thiserror` 2
- Binaries: `ocr-server` (API), `ocr-sandbox` (runner, egress proxy, tool guard), `ocr-bench` (benchmark reviewer)
- Bundled AI CLIs in image: Claude Code CLI 2.1.292, Codex CLI 0.163.0 (checksum-verified, `docker/Dockerfile`); GitHub `gh` from apt
## Configuration
- `OCR_*` variables set in `docker-compose.yml`: `OCR_BIND`, `OCR_DATA_DIR`, `OCR_DATABASE_URL_FILE`, `OCR_DATABASE_RO_URL_FILE`, `OCR_DATABASE_OWNER_URL_FILE`, `OCR_REPOS_DIR`, `OCR_AUTO_REINDEX`, `OCR_POLL_INTERVAL_SECONDS`, `OCR_WEB_APP_URL`, `OCR_REVIEW_RUNNER_DIR`, `OCR_REVIEW_RUNNERS` (default 20), `OCR_GH_HOST`, `OCR_API_TOKEN`, `GH_TOKEN` (dev fallback)
- Database URLs are read from files on the `database-secret` volume, never from plain env
- `.env` files exist only as local state and are not read
- User settings live in the database (Settings UI), not config files
- `package.json`, `tsconfig.json`, `vite.config.ts`, `biome.json`, `knip.json`
- `Cargo.toml`, `backend/Cargo.toml`, `deny.toml`
- `docker/Dockerfile*`, `docker/web-nginx.conf`
- `changelog.d/` fragments for release notes
## Platform Requirements
- Docker + Compose, pnpm, Rust 1.98.1, Postgres for Rust tests
- `pnpm debug:web` runs Vite on 1421 against the Docker backend
- Self-hosted Docker Compose; images published to `ghcr.io/bridglabs/opencodereview-{server,runner,web}` (`docker-compose.images.yml`)
- Web UI at `http://localhost:1421` (nginx proxying `/api` to server on 8787)
- License: PolyForm-Small-Business-1.0.0
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Patterns
- React components: PascalCase `.tsx` (`src/components/DesignSystem.tsx`, `src/platform/components/EnginePicker.tsx`).
- Non-component modules: camelCase `.ts` (`src/platform/lib/findings.ts`, `src/services/setupApiReviews.ts`).
- Tests co-located: `<name>.test.ts(x)` beside the source.
- snake_case `.rs` modules under `backend/src/<area>/`. Test modules are `<name>_tests.rs` or `tests.rs` / `tests/<topic>.rs`.
- TS: camelCase functions and variables, PascalCase types/interfaces (`FindingSelectionDraft`, `SetupStatus`). Plain exported functions (`export function selectedFindingIds(...)`), no classes.
- Rust: snake_case fns, CamelCase types, SCREAMING_CASE consts. Serde structs use `#[serde(rename_all = "camelCase")]` to match the TS types in `src/types`.
- Settings keys are snake_case strings (`review_engine_<provider>`, `review_lens_pass`).
## Code Style
## Import Organization
## Error Handling
- Rust: two styles coexist. Typed enums with `thiserror` for domain errors (`backend/src/review_archive/mod.rs` `Error`), a hand-written `Error` in `backend/src/db/pg/conn.rs` (mimics `rusqlite::Error`), and widespread `Result<T, String>` (hundreds of uses) for handler/job code with actionable messages. Errors reach HTTP responses via `to_string()`, so never embed credentials or DB URLs/row details.
- Invalid review reports fail; never relabel clean (see AGENTS.md Review invariants).
- Frontend: transport errors surface actionable Docker guidance, not "Failed to fetch" (`src/components/ErrorScreen.tsx`, `src/services/setupApiTransport.ts`).
## Logging
## Comments
## Function Design
## Module Design
## Commits and Releases
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## System Overview
```text
```
## Component Responsibilities
| Component | Responsibility | File |
|-----------|----------------|------|
| Server entry | CLI dispatch: serve, `archive-retention`, `import-sqlite` | `backend/src/main_server.rs` |
| Server runtime | Startup reconcile, worker threads, accept loop | `backend/src/server/mod.rs` (`run`) |
| Router | Auth check, `(method, path)` match, dynamic routes | `backend/src/server/routes.rs` |
| Review jobs | Start/supersede/cancel per-PR review threads | `backend/src/server/review_jobs.rs` |
| Polling | Detect new PR heads, queue reviews | `backend/src/server/polling.rs`, `polling_pr.rs` |
| Publication | Idempotent posting of validated selection | `backend/src/server/publication.rs` |
| Sign-in flows | Claude, Codex, GitHub auth | `backend/src/server/claude_auth.rs`, `codex_auth.rs` |
| Review orchestration | Scope, chunking, stages, final integration | `backend/src/review/execution.rs`, `chunks.rs`, `integration/` |
| Validation | Findings quote/anchor validation, output repair, lens | `backend/src/review/validation.rs`, `findings.rs`, `output_repair.rs`, `lens.rs` |
| Review archive | Frozen evidence, manifest, retention/purge | `backend/src/review_archive/` |
| Sandbox | Runner, egress allowlist, read commands, tool guard | `backend/src/sandbox/` |
| Providers | Claude/Codex CLI arguments, model list | `backend/src/providers/claude.rs`, `codex.rs`, `engines.rs` |
| Accounts | AI credentials, usage, sign-in projection | `backend/src/accounts/` |
| DB | Postgres access via pooled `Db` handle; one `Database` per step | `backend/src/db/` |
| GitHub | All calls via `gh` CLI | `backend/src/github/` |
| Indexer | File metadata, package/import graph, lightweight symbols | `backend/src/indexer/` |
| Bench | Headless reviewer for public benchmarks | `backend/src/bench/`, `backend/src/main_bench.rs` |
| Frontend routing | TanStack Router | `src/router.tsx` |
## Pattern Overview
- Synchronous, thread-based Rust (no async runtime); background jobs are spawned threads with cancel `AtomicBool`s.
- Server never runs the AI; it talks to `ocr-sandbox runner` over a Unix socket (`sandbox/runner.rs`, `MAX_FRAME`).
- Evidence is frozen into an archive before inference and every report is validated against it.
- Per-module `use super::*;` glob style in `server/` and `review/`; submodules re-exported with `pub(crate) use`.
- Executor fingerprint (`sandbox::engine_version`, hashes `review/*.rs` sources via `include_bytes!`) must match between backend and runner images.
## Layers
- Location: `src/`
- Contains: pages, components, hooks, API services
- Depends on: HTTP API only
- Used by: browser via nginx
- Location: `backend/src/server/`
- Contains: request parsing, routes, handlers, background workers, SSE streams (`stream.rs`)
- Depends on: `db`, `github`, `review`, `indexer`, `accounts`
- Used by: frontend
- Location: `backend/src/review/`, `backend/src/review_archive/`
- Contains: scope, evidence, chunks, validation, coverage, usage, policy
- Depends on: `providers`, `sandbox`, `db`, `github`
- Location: `backend/src/sandbox/`, `backend/src/providers/`
- Contains: runner, egress proxy, tool guard, CLI args
- Depends on: nothing from DB (never reaches the database)
- Location: `backend/src/db/`
- Contains: query modules per aggregate, migrations in `db/pg/`, `handle.rs` (`Db`)
- `db/schema.sql` and SQLite code exist only for `import-sqlite`.
## Data Flow
### Review Request Path
### Indexing Flow
- Durable state in Postgres; in-memory `ServerState` (`server/mod.rs`) holds active job maps. Multi-instance coordination through `Db::run_exclusive` with advisory lock keys in `server/runtime.rs` (`exclusive`), and instance heartbeats.
- Frontend: component state plus `src/services/setupApi*.ts`.
## Key Abstractions
## Entry Points
## Architectural Constraints
- **Threading:** thread per HTTP connection and per background job; shared state behind `Mutex`/`AtomicBool`.
- **Global state:** `sandbox::engine_version` uses `OnceLock`; `SharedState` (`Arc<ServerState>`).
- **Sandbox:** runners get only IPC socket and sign-in projection; no DB, repos or `ocr-data` mounts. Keep one runner per bench container.
- **Egress:** only AI provider hosts in `backend/src/sandbox/egress.rs`.
- **File size:** frontend <= 800 lines, Rust <= 2,500 (`pnpm check:file-size`).
- **Migrations:** append only in `backend/src/db/pg/`, list in `db/migrations.rs`/`pg/migrate.rs`.
- **Transactions:** call `lock_pull_request` before a read-then-write on PR state.
## Anti-Patterns
### In-process AI fallback
### Stale refs in scope
### Holding `Database` across slow calls
## Error Handling
## Cross-Cutting Concerns
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

| Skill | Description | Path |
|-------|-------------|------|
| comments | Fetch review feedback on the current branch's pull request, apply valid fixes as focused commits, push, reply with commit links and resolve fixed threads. | `.agents/skills/comments/SKILL.md` |
| git | Commit, push and pull request operations for OpenCodeReview, using Conventional Commits and the repository PR template. | `.agents/skills/git/SKILL.md` |
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
