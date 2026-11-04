---
last_mapped_commit: 1a47d4f35f3f87acdd5dd4e1767477628dd27a25
---
<!-- refreshed: 2026-11-04 -->
# Architecture

**Analysis Date:** 2026-11-04

## System Overview

```text
┌─────────────────────────────────────────────────────────────┐
│  Browser: React SPA (Vite)  `src/`                           │
│  Onboarding `src/pages/`  Platform `src/platform/pages/`     │
│  API client `src/services/setupApi*.ts`                      │
└──────────────────────────┬──────────────────────────────────┘
                           │ /api (nginx `docker/web-nginx.conf`, port 1421)
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  ocr-server  `backend/src/main_server.rs`                    │
│  Hand-rolled HTTP (std TcpListener, thread per connection)   │
│  `server/http.rs` -> `server/routes.rs` -> handler modules   │
│  Background workers: poll, queue drain, reindex, retention   │
└───────┬───────────────┬───────────────┬─────────────────────┘
        │               │               │
        ▼               ▼               ▼
┌──────────────┐ ┌──────────────┐ ┌─────────────────────────────┐
│ `db/` Postgres│ │ `github/` gh │ │ `review/` orchestration     │
│ `db/pg/*.sql` │ │ `indexer/`   │ │ `review_archive/` frozen    │
└──────────────┘ └──────────────┘ │ evidence                    │
                                   └──────────────┬──────────────┘
                                                  │ Unix socket IPC (framed)
                                                  ▼
┌─────────────────────────────────────────────────────────────┐
│  ocr-sandbox runner (review-runner replicas)                 │
│  `sandbox/runner.rs`: drops to UID 10001, runs claude/codex  │
│  `providers/`, tool-guard hook; egress proxy `sandbox/egress.rs` │
└─────────────────────────────────────────────────────────────┘
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

**Overall:** Dockerized multi-service monolith. One Rust library crate (`opencodereview_server`) with three binaries; React SPA frontend; sandboxed AI execution over IPC.

**Key Characteristics:**
- Synchronous, thread-based Rust (no async runtime); background jobs are spawned threads with cancel `AtomicBool`s.
- Server never runs the AI; it talks to `ocr-sandbox runner` over a Unix socket (`sandbox/runner.rs`, `MAX_FRAME`).
- Evidence is frozen into an archive before inference and every report is validated against it.
- Per-module `use super::*;` glob style in `server/` and `review/`; submodules re-exported with `pub(crate) use`.
- Executor fingerprint (`sandbox::engine_version`, hashes `review/*.rs` sources via `include_bytes!`) must match between backend and runner images.

## Layers

**Frontend:**
- Location: `src/`
- Contains: pages, components, hooks, API services
- Depends on: HTTP API only
- Used by: browser via nginx

**HTTP/API:**
- Location: `backend/src/server/`
- Contains: request parsing, routes, handlers, background workers, SSE streams (`stream.rs`)
- Depends on: `db`, `github`, `review`, `indexer`, `accounts`
- Used by: frontend

**Domain (review):**
- Location: `backend/src/review/`, `backend/src/review_archive/`
- Contains: scope, evidence, chunks, validation, coverage, usage, policy
- Depends on: `providers`, `sandbox`, `db`, `github`

**Execution sandbox:**
- Location: `backend/src/sandbox/`, `backend/src/providers/`
- Contains: runner, egress proxy, tool guard, CLI args
- Depends on: nothing from DB (never reaches the database)

**Persistence:**
- Location: `backend/src/db/`
- Contains: query modules per aggregate, migrations in `db/pg/`, `handle.rs` (`Db`)
- `db/schema.sql` and SQLite code exist only for `import-sqlite`.

## Data Flow

### Review Request Path

1. Poll scheduler or HTTP request finds a PR needing review (`server/polling.rs`, `server/review_http.rs`).
2. `start_review_job_with_intent` registers a `ReviewJob` in `state.review_jobs` and spawns a thread (`server/review_jobs.rs`).
3. `review::execute` (`review/execution.rs`) fetches PR head via git, computes scope vs fresh `origin/<base>`, freezes evidence (`review_archive/`).
4. Chunks run as stages in the sandbox runner (`review/chunks.rs`, `sandbox/runner.rs`), each report validated (`review/validation.rs`), optional repair/lens.
5. Final integration batches merge candidates (`review/integration/`).
6. Result stored in Postgres (`db/review_results.rs`); publication posts comments via `gh` (`server/publication.rs`, `github/comments.rs`).
7. UI follows progress via SSE (`server/stream.rs`, `src/platform/hooks/useReviewRunStream.ts`).

### Indexing Flow

1. UI triggers indexing (`server/indexing.rs`); `indexer/` builds files, packages, import edges, symbols; progress via `IndexProgressEvent` (`state.rs`).

**State Management:**
- Durable state in Postgres; in-memory `ServerState` (`server/mod.rs`) holds active job maps. Multi-instance coordination through `Db::run_exclusive` with advisory lock keys in `server/runtime.rs` (`exclusive`), and instance heartbeats.
- Frontend: component state plus `src/services/setupApi*.ts`.

## Key Abstractions

**`Db` / `Database`:** `backend/src/db/handle.rs`, `db/mod.rs`; open one per step, never held across Git/GitHub/model calls.
**`ReviewInputs` / `ReviewOutput`:** inputs/outputs of `review::execute`.
**Frozen archive:** `backend/src/review_archive/mod.rs` (`VERSION`, `POLICY_VERSION`; stored archives validate under their recorded policy).
**Engines:** `backend/src/providers/engines.rs`, first entry is the default.
**Built-in skill:** `backend/src/assets/review-skill.md`.

## Entry Points

**`ocr-server`:** `backend/src/main_server.rs`; serves API on `127.0.0.1:8787`; also `archive-retention`, `import-sqlite`.
**`ocr-sandbox`:** `backend/src/main_sandbox.rs`; `runner|egress|engine-version|tool-guard`.
**`ocr-bench`:** `backend/src/main_bench.rs`; benchmark image only (`docker/Dockerfile.bench`).
**Web:** `src/main.tsx` -> `src/App.tsx` -> `src/router.tsx`.

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
**What happens:** running the model inside `server`.
**Do this instead:** go through the runner IPC (`sandbox/runner.rs`).

### Stale refs in scope
**What happens:** falling back to local/default branch when fetch fails.
**Do this instead:** fail preparation (`review/scope.rs`).

### Holding `Database` across slow calls
**Do this instead:** `db_handle.open()` per step.

## Error Handling

**Strategy:** `Result<_, String>` everywhere; HTTP errors via `json_error(status, msg)` (`server/response.rs`). Invalid reports fail, never relabelled clean; invalid points dropped with warnings (`review/warnings.rs`). No automatic re-run after startup failure.

## Cross-Cutting Concerns

**Logging:** `eprintln!`/`println!`; per-run `harness_log` in run dir.
**Validation:** quote/anchor validation against the frozen snapshot (`review/validation.rs`, `findings.rs`).
**Authentication:** API token (`OCR_API_TOKEN`) checked in `routes.rs` (`check_auth`, `is_public_route`); GitHub via `gh`.

---

*Architecture analysis: 2026-11-04*
