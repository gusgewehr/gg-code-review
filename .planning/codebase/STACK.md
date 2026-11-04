---
last_mapped_commit: 1a47d4f35f3f87acdd5dd4e1767477628dd27a25
---
# Technology Stack

**Analysis Date:** 2026-11-04

## Languages

**Primary:**
- Rust (edition 2021, toolchain 1.98.1) - backend in `backend/src/` (crate `opencodereview-server`, workspace root `Cargo.toml`)
- TypeScript ~5.9 - React frontend in `src/`, `src/platform/`

**Secondary:**
- Python 3 - Docker smoke/diagnostic scripts in `docker/*.py`, benchmark tooling `bench/report.py`
- Node (ESM `.mjs`) - repo scripts in `scripts/` (file-size check, changelog, site build, OG images)
- SQL - Postgres migrations in `backend/src/db/pg/*.sql`
- Shell - `docker/bench-entrypoint.sh`, `docker/web-entrypoint.sh`, `docker/postgres/init-roles.sh`

## Runtime

**Environment:**
- Docker Compose is the only product runtime (`docker-compose.yml`, `docker-compose.images.yml`, `docker-compose.scale.yml`)
- Server/runner images: `debian:bookworm-slim` over `rust:1.98.1-bookworm` build stage (`docker/Dockerfile`, `docker/Dockerfile.runner`)
- Web image: `node:22-bookworm-slim` build, `nginx:1.30-alpine` serve (`docker/Dockerfile.web`)
- Database: `postgres:17` pinned by digest
- Sandbox uses `bubblewrap` in the image

**Package Manager:**
- pnpm 10.15.0 (`packageManager` in `package.json`); lockfile `pnpm-lock.yaml` present
- Cargo, lockfile `Cargo.lock` present (`--locked` everywhere); `deny.toml` for cargo-deny

## Frameworks

**Core:**
- React 19.3 + react-dom - UI
- TanStack Router 1.170 / React Query 5 - routing and server state
- Tailwind CSS 4.3 (`@tailwindcss/vite`), theme tokens in `src/styles/globals.css`
- TipTap 3.32 (editor), `@xyflow/react` 12.12, `react-force-graph-2d`, `recharts`, `react-markdown` + `remark-gfm` + `rehype-raw`, `nuqs`, `lucide-react`
- Backend: no web framework; custom HTTP server in `backend/src/server/http.rs`, `routes.rs`, with `reqwest` (blocking) for outbound HTTP and thread pools (`scheduled-thread-pool`, `r2d2`)

**Testing:**
- Vitest 4.1 with jsdom 26 (`vite.config.ts`)
- Rust `cargo test` (needs `OCR_TEST_DATABASE_URL`)
- Playwright 1.64 (devDependency)

**Build/Dev:**
- Vite 7.3 (`vite.config.ts`, dev port 1420/1421), `tsc`
- Biome 2.6.4 lint/format (`biome.json`: 2 spaces, width 120, double quotes)
- Knip (`knip.json`), clippy `-D warnings`, rustfmt

## Key Dependencies

**Critical (`backend/Cargo.toml`):**
- `postgres` 0.19 + `r2d2_postgres` + `tokio-postgres-rustls` - Postgres access with TLS
- `rusqlite` 0.32 (bundled) - only for `ocr-server import-sqlite`
- `reqwest` 0.13 (blocking, form, json), `rustls` 0.23 + `rustls-native-certs`
- `serde`/`serde_json`, `sha2`, `base64`, `libc`, `pulldown-cmark`, `thiserror` 2

**Infrastructure:**
- Binaries: `ocr-server` (API), `ocr-sandbox` (runner, egress proxy, tool guard), `ocr-bench` (benchmark reviewer)
- Bundled AI CLIs in image: Claude Code CLI 2.1.292, Codex CLI 0.163.0 (checksum-verified, `docker/Dockerfile`); GitHub `gh` from apt

## Configuration

**Environment:**
- `OCR_*` variables set in `docker-compose.yml`: `OCR_BIND`, `OCR_DATA_DIR`, `OCR_DATABASE_URL_FILE`, `OCR_DATABASE_RO_URL_FILE`, `OCR_DATABASE_OWNER_URL_FILE`, `OCR_REPOS_DIR`, `OCR_AUTO_REINDEX`, `OCR_POLL_INTERVAL_SECONDS`, `OCR_WEB_APP_URL`, `OCR_REVIEW_RUNNER_DIR`, `OCR_REVIEW_RUNNERS` (default 20), `OCR_GH_HOST`, `OCR_API_TOKEN`, `GH_TOKEN` (dev fallback)
- Database URLs are read from files on the `database-secret` volume, never from plain env
- `.env` files exist only as local state and are not read
- User settings live in the database (Settings UI), not config files

**Build:**
- `package.json`, `tsconfig.json`, `vite.config.ts`, `biome.json`, `knip.json`
- `Cargo.toml`, `backend/Cargo.toml`, `deny.toml`
- `docker/Dockerfile*`, `docker/web-nginx.conf`
- `changelog.d/` fragments for release notes

## Platform Requirements

**Development:**
- Docker + Compose, pnpm, Rust 1.98.1, Postgres for Rust tests
- `pnpm debug:web` runs Vite on 1421 against the Docker backend

**Production:**
- Self-hosted Docker Compose; images published to `ghcr.io/bridglabs/opencodereview-{server,runner,web}` (`docker-compose.images.yml`)
- Web UI at `http://localhost:1421` (nginx proxying `/api` to server on 8787)
- License: PolyForm-Small-Business-1.0.0

---

*Stack analysis: 2026-11-04*
