---
last_mapped_commit: 1a47d4f35f3f87acdd5dd4e1767477628dd27a25
---
# Testing Patterns

**Analysis Date:** 2026-11-04

## Test Framework

**Frontend:** Vitest with jsdom (`vite.config.ts`: `test: { environment: "jsdom" }`). No Testing Library; tests render with `react-dom/client` `createRoot` inside `act`.
**Backend:** Rust built-in `#[test]` via `cargo test`; Postgres-backed tests use `backend/src/db/pg/test_support.rs`.
**Docker/scripts:** Python `unittest` (`docker/test_diagnostics.py`, `docker/test-web-proxy.py`), smoke scripts (`docker/runner-smoke.py`, `docker/bench-smoke.py`), and `scripts/dev-web-docker.test.ts` (Vitest).

**Run Commands:**
```bash
pnpm test                                   # vitest run (all frontend tests)
cargo test --workspace --locked             # all Rust tests
cargo test -p opencodereview-server <filter> --lib --locked
python3 -B docker/test_diagnostics.py
python3 -B docker/test-web-proxy.py
python3 -u docker/runner-smoke.py           # after building images; fake inference, no paid calls
```
Rust DB tests need `OCR_TEST_DATABASE_URL` (e.g. `postgres://ocr:dev@127.0.0.1:55432/postgres`, role must be able to create databases). Zero matching tests is not a pass. No coverage tooling or threshold is enforced.

## Test File Organization

- Frontend: co-located `Name.test.ts(x)` next to source in `src/components/`, `src/pages/`, `src/platform/{components,hooks,lib}/`, `src/services/`, plus `src/RootLayout.test.tsx`.
- Rust: separate files attached from the module under test, either `#[cfg(test)] mod x_tests;`, `#[path = "findings_tests.rs"] mod decision_tests;` (`backend/src/review/findings.rs`), or `#[path = "tests/usage.rs"]` (`backend/src/review/tests/`). Tests start with `use super::*;`. Large areas: `backend/src/db/*_tests.rs`, `backend/src/review/*_tests.rs`, `backend/src/review/integration/tests.rs`, `backend/src/review_archive/tests.rs`, `backend/src/github/tests.rs`, `backend/src/indexer/tests.rs`.

## Test Structure

Frontend pure-logic tests use `describe` + `it`/`it.each` with small factory helpers and typed fixtures (`src/platform/lib/findings.test.ts`):
```typescript
const candidate = (id: string, selected: boolean, blockedReason: string | null = null): FindingCandidate => ({ ... });
describe("review point classification", () => {
  it.each([["No findings.", "unknown"]])("classifies all unfenced points: %s", (body, expected) => {
    expect(reviewPointKind(body)).toBe(expected);
  });
});
```
Component tests are top-level `it(...)` with `afterEach` cleanup (`src/components/Toast.test.tsx`):
```typescript
const root = createRoot(document.createElement("div"));
await act(async () => root.render(<Toaster />));
await act(async () => vi.advanceTimersByTime(3500));
await act(async () => root.unmount());
```
Query the DOM directly (`container.querySelector('[role="status"]')`). Pages needing React Query wrap in `QueryClientProvider` (`src/platform/components/EnginePicker.test.tsx`). Set `vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true)` where act warnings appear (`src/pages/Onboarding.test.tsx`).

Rust tests are plain descriptive-sentence functions, table-driven with `for (input, expected) in [...]`, `assert!`/`assert_eq!`/`matches!`, error messages passed as `assert!(cond, "{error}")`; `serde_json::json!` builds inputs.

## Mocking

**Frontend:** `vi.mock` of the API module with `vi.fn()` members, then `vi.mocked(api.fn).mockResolvedValue(...)` / `mockRejectedValue(new TypeError("Failed to fetch"))`; router mocked via `vi.mock("@tanstack/react-router", ...)` (`src/pages/Onboarding.test.tsx`). Fake timers via `vi.useFakeTimers()` reset in `afterEach`.

**Backend:** No mocking framework. Use real Postgres per test (`TestDb::new()` clones a migrated template DB, truncates and reuses finished DBs; `db.pool()`, `db.client()`, `db.settings()`), synthetic Git archives built in-process (`archive(...)` in `backend/src/review/integration/tests.rs`), shared helpers like `crate::review::usage_tests::review_inputs`. Fake inference is used in `docker/runner-smoke.py`.

**Do mock:** HTTP/API service modules in UI tests, timers. **Do not mock:** the database (use `TestDb`), pure logic in `src/platform/lib`. Never make paid AI calls; real review diagnostics need explicit maintainer authorization.

## Fixtures and Factories

Inline factory helpers at the top of each test file (`account(...)`, `candidate(...)`, `engines` arrays) and constants such as `HEAD`/`BASE` 40-char SHAs in Rust. No shared fixtures directory; Rust shared helpers live in `*_tests`/`test_support.rs` modules (`pub(crate)`). Benchmark manifests/adapters are in `bench/`.

## Coverage

None enforced. Add tests for new non-trivial logic in the same layer: pure TS in `src/platform/lib/*.test.ts`, Rust beside the module via a `_tests.rs` file.

## Test Types

- Unit: TS lib functions, Rust validators (findings, evidence, coverage, usage).
- Integration: Rust against Postgres (`backend/src/db/concurrency_tests.rs`, `backend/src/db/pg/tests.rs`), review pipeline (`backend/src/review/integration/tests.rs`), component tests with mocked APIs.
- Smoke/E2E: Docker-based (`docker/runner-smoke.py`, `docker/bench-smoke.py`, `docker-smoke.yml` workflow); no browser E2E framework.
- CI (`.github/workflows/ci.yml`): `pnpm lint`, `knip`, `typecheck`, `test`, `build`, `cargo fmt --check`, clippy `-D warnings`, cargo tests, Python tests, file-size check, `docker compose config --quiet`.

## Common Patterns

**Async (TS):** `await act(async () => ...)` around renders, events and timer advances.
**Error (TS):** `mockRejectedValue(new TypeError("Failed to fetch"))` then assert the Docker guidance renders.
**Error (Rust):** `let error = f().unwrap_err(); assert!(error.contains("..."), "{error}");` and `matches!(res, Err(Error::Variant))`.

---

*Testing analysis: 2026-11-04*
