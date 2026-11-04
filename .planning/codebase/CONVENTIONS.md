---
last_mapped_commit: 1a47d4f35f3f87acdd5dd4e1767477628dd27a25
---
# Coding Conventions

**Analysis Date:** 2026-11-04

Authoritative guide: `AGENTS.md` (ground rules, UI rules, sandbox and review invariants), `CONTRIBUTING.md`. No project skills in `.claude/skills/`; agent workflows live in `.agents/skills/git/SKILL.md` and `.agents/skills/comments/SKILL.md`.

## Naming Patterns

**Files (frontend):**
- React components: PascalCase `.tsx` (`src/components/DesignSystem.tsx`, `src/platform/components/EnginePicker.tsx`).
- Non-component modules: camelCase `.ts` (`src/platform/lib/findings.ts`, `src/services/setupApiReviews.ts`).
- Tests co-located: `<name>.test.ts(x)` beside the source.

**Files (backend):**
- snake_case `.rs` modules under `backend/src/<area>/`. Test modules are `<name>_tests.rs` or `tests.rs` / `tests/<topic>.rs`.

**Functions/variables:**
- TS: camelCase functions and variables, PascalCase types/interfaces (`FindingSelectionDraft`, `SetupStatus`). Plain exported functions (`export function selectedFindingIds(...)`), no classes.
- Rust: snake_case fns, CamelCase types, SCREAMING_CASE consts. Serde structs use `#[serde(rename_all = "camelCase")]` to match the TS types in `src/types`.
- Settings keys are snake_case strings (`review_engine_<provider>`, `review_lens_pass`).

## Code Style

**Frontend formatting/linting:** Biome 2 (`biome.json`): 2-space indent, line width 120, double quotes, trailing commas `all`, `recommended` lint preset, organized imports. Run `pnpm lint:fix`; CI runs `pnpm lint`. `pnpm knip` flags unused files/exports/deps (`knip.json`). TypeScript strict-checked via `pnpm typecheck`. `noNonNullAssertion` is off only in `*.test.ts(x)`.

**Backend:** `cargo fmt --all` with `backend/rustfmt.toml` (`max_width = 120`); `cargo clippy --workspace --all-targets --locked -- -D warnings`. `too_many_arguments` and `type_complexity` are allowed in `backend/Cargo.toml`. `cargo deny check` when `Cargo.lock` changes.

**Hooks:** `.githooks/` pre-commit formats staged files (Biome, rustfmt); installed by `pnpm install` (`prepare`).

**File size limits** (`pnpm check:file-size`, `scripts/check-file-sizes.mjs`): frontend <= 800 lines, Rust <= 2,500. Split modules early (e.g. `src/services/setupApi*.ts` composed into `setupApi`).

**UI rules:** Tailwind utilities and tokens from `src/styles/globals.css`, no raw hex; reuse `src/components/DesignSystem.tsx`; `src/styles/components.css` only for third-party internals; loading uses `src/components/BrandLoading.tsx`; max two visible buttons per action area, extras in a `DSMenu`.

## Import Organization

TS (Biome-organized): external packages first, then relative imports, type-only imports use `import type`. No path aliases; relative paths (`../../types`).
Rust: `std`, external crates, then `crate::`/`super::`; tests start with `use super::*;`.

## Error Handling

- Rust: two styles coexist. Typed enums with `thiserror` for domain errors (`backend/src/review_archive/mod.rs` `Error`), a hand-written `Error` in `backend/src/db/pg/conn.rs` (mimics `rusqlite::Error`), and widespread `Result<T, String>` (hundreds of uses) for handler/job code with actionable messages. Errors reach HTTP responses via `to_string()`, so never embed credentials or DB URLs/row details.
- Invalid review reports fail; never relabel clean (see AGENTS.md Review invariants).
- Frontend: transport errors surface actionable Docker guidance, not "Failed to fetch" (`src/components/ErrorScreen.tsx`, `src/services/setupApiTransport.ts`).

## Logging

Rust uses `eprintln!`-style output (no tracing framework); never log secrets or credential bytes. Frontend: user-visible feedback via `toast` from `src/components/Toast.tsx`.

## Comments

Add only where the code needs it; explain the why. Rust uses `///` doc comments on non-obvious fields/types and `//!` module headers where behavior is intricate (`backend/src/db/pg/test_support.rs`). TS uses `/** ... */` sparingly (`SetupStatus.nextStep`).

## Function Design

Compact, minimal nesting; handlers pass state as plain arguments. Pure logic lives in `src/platform/lib/*.ts` (testable without React). Every SQL value is bound as a parameter; open one `Database` per step, never across Git/GitHub/model calls; `lock_pull_request` before state-dependent writes; `Db::run_exclusive` for singleton background work.

## Module Design

TS: named exports, no default exports, no barrel `index.ts`; API objects are composed by spreading (`setupApi` in `src/services/setupApi.ts`). Rust: `mod.rs` per area with submodules; test modules attached via `#[cfg(test)] mod x_tests;` or `#[path = "tests/x.rs"]`.

## Commits and Releases

Conventional Commits for commits and PR titles (`.github/workflows/pr-title.yml`). User-visible changes need `changelog.d/<branch>.md` (`pnpm changelog:check`); never edit `CHANGELOG.md`. No AI attribution in commits/PRs per `AGENTS.md`. pnpm only. Migrations: append SQL files to `backend/src/db/pg/` and list in `migrate.rs`; never edit applied ones.

---

*Convention analysis: 2026-11-04*
