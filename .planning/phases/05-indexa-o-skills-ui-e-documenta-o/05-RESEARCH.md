# Phase 5: Indexação, skills, UI e documentação - Research

**Researched:** 2026-11-05
**Domain:** forge parity (Rust backend gate removal + React vocabulary swap + docs)
**Confidence:** HIGH (all findings from codebase grep; no external libs)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Indexação usa o clone/fetch via forge (mesma auth git da Fase 3). Nada específico de GitLab além da clone URL `https://<host>/<path_with_namespace>.git` e do default branch vindo da API.
- Skills: só garantir que nada assume `owner/name` de dois segmentos nem `github.com`.
- Payload de status/setup ganha `forge` (`github`|`gitlab`) e `forgeHost`; o frontend lê isso uma vez pelo serviço de setup existente. (UI-SPEC refines: reuse `GET /forge/status`/`ForgeStatus`, add fields there only.)
- Em deploy GitLab: "pull request" vira "merge request", `#123` vira `!123`, marca GitHub vira texto "GitLab" (ícone genérico lucide `GitMerge`), links usam URLs vindas do backend.
- Helper `forgeTerms(forge)` com as strings; deploy GitHub idêntico.
- Mensagens de erro de sign-in citam `glab`/PAT em deploy GitLab.
- `docs/gitlab.md` (ou seção no README): `OCR_FORGE`, `OCR_GITLAB_HOST`, montagem da CA, PAT (escopo `api`, bot recomendado), login em Settings, script de spike, limitações da v1.
- Fragmento em `changelog.d/` no formato existente.

### Claude's Discretion
Componentes exatos tocados e texto da doc. Velocidade acima de polimento.

### Deferred Ideas (OUT OF SCOPE)
- PR upstream para a BridgLabs (v2).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| IDX-01 | Indexação de projeto GitLab (metadados, grafo, símbolos) | One-line gate in `server/mod.rs:291-295`; clone already forge-aware (`indexing.rs:439`) |
| IDX-02 | Skills funcionam em projetos GitLab | Skills are forge-agnostic DB review-methodology versions; verification only |
| UI-01 | UI diz "merge request", `!iid`, links GitLab | `forgeTerms.ts` + `useForgeTerms`, touch listed files |
| UI-02 | Docs + changelog | `docs/gitlab.md` (new dir), `changelog.d/feat-gitlab-parity.md` |
</phase_requirements>

## Summary

Indexing is gated off in exactly one place: `backend/src/server/mod.rs:291-295` — `let github = forge::current() == Forge::GitHub; // GitLab indexing lands in Phase 5; if config.auto_reindex && github { start_reindex_worker(...) }`. Everything downstream is already forge-aware: `ensure_repo_checkout_cancellable` (`server/indexing.rs:439`) clones `https://{forge::gh_host()}/{repo.full_name}.git` (`gh_host()` returns the pinned GitLab host, `forge/mod.rs:153-158`); `indexer::git` applies `forge::apply_git_https_auth` (`indexer/mod.rs:620`, GitLab branch → `gitlab::apply_git_auth`, `forge/mod.rs:160-165`); default branch is stored from API (`gitlab/projects.rs:19,76` `default_branch`) with fallback to `local::default_branch` (`indexing.rs:150-154`). Manual indexing routes are not gated (no `require_github` in `server/`). So IDX-01 = remove the gate + one test proving the worker/clone path works with GitLab full_name.

Nested namespaces: GitLab `RepoInput.owner = namespace` (may be `group/sub`), `name = path`, `full_name = path_with_namespace` (`projects.rs:70-76`). Checkout path is `repos_dir/owner/name` (`indexing.rs:410`) which nests dirs fine, and the clone URL uses `full_name` (correct). Risk: `db/repositories.rs:6` `rsplit_once('/')` (fallback only), `review_archive/retention.rs:454` and `bench/mod.rs:121` `split_once('/')` — check retention.rs:454 for owner/name assumptions (not examined in depth; verify in plan).

Pinned-dependency capture is skipped on GitLab at `review/frozen.rs:58-59` (shortcut comment; `review/dependencies.rs` is gh-only: "authenticated gh read access"). Not required by IDX-01/IDX-02: leave as a documented v1 limitation in `docs/gitlab.md` ("submodules and linked-repo pinned dependencies are not captured on GitLab; the review uses the checkout's own evidence"). Do NOT implement.

Skills (`routes.rs:57-108`, `review_jobs.rs:78-101` `db.active_skill_content()`, `assets/review-skill.md`, `src/platform/lib/skillEditor.ts`) are DB-stored review-methodology versions copied into the review workspace as `skill/SKILL.md`. No forge/owner-name dependency. IDX-02 = grep verification plus (optionally) one test; no code change expected. The skill markdown/prompt text mentions "PR"/"GitHub" only generically (prompt in `review/evidence.rs:1557` says "PR.md"); leave as-is.

**Primary recommendation:** 3 small plans: (1) backend: remove gate + add `forge`/`forgeHost` alias fields (optional, see below) + test; (2) frontend `forgeTerms.ts`/`useForgeTerms` and string routing; (3) `docs/gitlab.md` + README pointer + changelog fragment.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary | Rationale |
|---|---|---|---|
| Index worker start | API/Backend (`server/mod.rs`) | — | gate lives at startup |
| Clone/fetch auth | Backend (`forge`, `gitlab::apply_git_auth`) | — | runners never get creds |
| Vocabulary/ref format | Browser (`forgeTerms.ts`) | Backend supplies `provider` | pure presentation |
| Links | Backend (URL in data) | Browser renders | no github.com built in frontend |

## Standard Stack

No new dependencies. Use existing: `@tanstack/react-query` (`useQuery`), `lucide-react` (`GitMerge`), Vitest, `cargo test`. No package legitimacy audit needed (no installs).

## Architecture Patterns

### Frontend forge source (already present)
- `GET /forge/status` → `routes.rs:24,194-198 forge_status()` returns `ForgeStatus {provider, host, loggedIn, login, expiresAt, error}` (`src/types/settings.ts:140`); `setupApi.forgeStatus` (`setupApiAccess.ts:40`); query key `["forge","status"]` used in `SettingsPage.tsx:39`. `provider` and `host` already satisfy CONTEXT's `forge`/`forgeHost`; **do not add new fields or endpoints** (decide: reuse `provider`/`host`).
- `useForgeTerms()` = `useQuery({queryKey:["forge","status"], queryFn: setupApi.forgeStatus, retry:false, staleTime: Infinity})` → `forgeTerms(data?.provider ?? "github")`. Same key as SettingsPage so cache is shared (match SettingsPage's options to avoid refetch churn).
- `forgeTerms` returns object with: `pr`, `prs`, `Pr`, `PullRequest(s)`, `ref(n)`, `platform` ("GitHub"/"GitLab"), `openOn`, etc. Snapshot test for github.

### Files to touch (from `rg`, counts of matches)
`ReviewsPage.tsx`(12; near 800-line cap — only replace literals with `t.x`, keep line count flat, no new imports beyond hook+GitMerge), `reviews/ReviewDetail.tsx`(14), `reviews/ReviewSections.tsx`, `reviews/RunDetails.tsx`(3), `usage/UsageTotals.tsx`, `usage/PrUsageList.tsx`, `SettingsPage.tsx`(15, labels only), `settings/ReviewSettingsPanel.tsx`(8), `GitHubMark` users: only `ReviewsPage.tsx:659` and `ReviewDetail.tsx:138`. External link at `ReviewsPage.tsx:650-660` uses `monitoring.url` (backend data; no github.com built in `src/platform` — rg `github\.com` returned nothing there). Onboarding pages (`src/pages/Onboarding*.tsx`, `AccessConnect.tsx`) contain GitHub text; UI-SPEC scope lists Reviews/detail/run/usage/sign-in only → Onboarding out of scope unless GitLab onboarding is already branched (Phase 2); executor should rg and route only user-visible strings reachable in GitLab deploys.
- Check file sizes with `pnpm check:file-size` after edits.

### Sign-in error copy
GitLab auth errors: backend/`GitLabConnection.tsx` (Phase 2) — executor greps for "GitHub authorization" and ensures the GitLab message from UI-SPEC row 62 is used where an auth error is shown for gitlab.

## Don't Hand-Roll
| Problem | Use Instead |
|---|---|
| New forge endpoint | existing `/forge/status` |
| GitLab brand logo | lucide `GitMerge` |
| Clone auth | `forge::apply_git_https_auth` |

## Common Pitfalls
1. **Removing the gate but unused var**: `github` bool at `mod.rs:291` becomes unused → delete it (clippy `-D warnings`).
2. **ReviewsPage.tsx size cap** (800): keep net lines ≤ current; `pnpm check:file-size`.
3. **Search matching** `ReviewsPage.tsx:377`: GitLab must match `!n` and bare `n`.
4. **Default `github` while loading** — never flip GitHub text; tests mock `forgeStatus` (SettingsPage.test mocks `githubForge`); components that call `useForgeTerms` in tests lacking a QueryClientProvider/`forgeStatus` mock will break existing tests — wrap or default to github when query errors; verify existing tests still pass (baseline: 20 pre-existing `localStorage.clear()` failures under Node 25).
5. **Reindex worker on GitLab**: `reindex_pass` (`indexing.rs:342-395`) calls `remote_default_sha` with git fetch → needs auth already provided by `indexer::git`. Self-signed CA: git needs the CA; Phase 2/3 `gitlab::ca::prepare`/`apply_git_auth` presumably set `GIT_SSL_CAINFO` — confirm in `gitlab/` (not re-verified here).
6. Dependency capture skip at `frozen.rs:58` stays; update nothing.

## Code Examples
```rust
// backend/src/server/mod.rs (replace lines 291-295)
if config.auto_reindex {
    start_reindex_worker(state.clone());
}
```
```ts
// src/platform/lib/forgeTerms.ts
export type ForgeKind = "github" | "gitlab";
export function forgeTerms(forge: ForgeKind) {
  const gl = forge === "gitlab";
  return {
    platform: gl ? "GitLab" : "GitHub",
    pr: gl ? "merge request" : "pull request",
    prShort: gl ? "MR" : "PR",
    ref: (n: number | string) => `${gl ? "!" : "#"}${n}`,
  };
}
```

## Docs and release
- `docs/` does not exist → create `docs/gitlab.md`; add one README link line (README currently GitHub-only: lines 9, 11, 25, 35, 66-75 env table; add `OCR_FORGE`/`OCR_GITLAB_HOST`/`OCR_GITLAB_CA_FILE` pointer, not a rewrite).
- Env vars confirmed by `changelog.d/feat-gitlab-connection.md`: `OCR_FORGE=gitlab`, `OCR_GITLAB_HOST` (bare host, no scheme/path — `forge/mod.rs:52-58`), `OCR_GITLAB_CA_FILE` (server-only mount), PAT scope `api` in Settings > Integrations, repos in Settings > Repository access, forge/host pinned on first start (`forge/guard.rs`). Spike script name/location: executor must `ls docker/*.py scripts/` or Phase 1-4 SUMMARY to cite correctly (not verified here).
- Limitations to document: no approve/request-changes/merge (request_changes publishes comment), pinned dependencies not captured, polling only (no webhooks), one provider per deploy, Developer+ access required, `glab` only in server image.
- Changelog: `changelog.d/feat-gitlab-connection.md` exists (format: `### Added` bullets, 2-space continuation, `### Changed`). Add `changelog.d/feat-gitlab-parity.md` (file named after branch with `-`; check script `pnpm changelog:check` → `scripts/changelog.mjs check`). Content: Added — indexing and skills on GitLab, UI uses merge request terms and `!iid`, docs link.

## Assumptions Log
| # | Claim | Risk |
|---|---|---|
| A1 | `apply_git_auth` handles the internal CA for git (`GIT_SSL_CAINFO`) | Indexing clone fails on self-signed GitLab; executor verify in `gitlab/` |
| A2 | `retention.rs:454` split_once('/') not a problem for nested namespaces | archive retention mis-parses `group/sub/proj`; check |
| A3 | Onboarding pages out of UI scope | minor wording gap |

## Open Questions
1. Spike script path for docs — grep during execution. Recommendation: cite whichever exists; omit if none.

## Environment Availability
Skipped for code: needs Postgres (`OCR_TEST_DATABASE_URL`) for cargo tests; pnpm for vitest — same as baseline.

## Validation Architecture

### Test Framework
| Property | Value |
|---|---|
| Framework | cargo test (needs `OCR_TEST_DATABASE_URL`); Vitest 4.1 + jsdom |
| Config | `vite.config.ts` |
| Quick run | `cargo test -p opencodereview-server indexing` ; `pnpm vitest run src/platform/lib/forgeTerms.test.ts` |
| Full suite | `cargo test --locked && pnpm test && pnpm check:file-size && pnpm changelog:check && cargo clippy --locked -- -D warnings` |

### Phase Requirements → Test Map
| Req | Behavior | Type | Command | Exists |
|---|---|---|---|---|
| IDX-01 | no GitHub gate: reindex worker started for GitLab; clone URL for nested `group/sub/proj` uses forge host | unit (extract pure fn e.g. `clone_url(host, full_name)` and test; plus grep assert no `GitHub` gate) | `cargo test -p opencodereview-server clone_url` | ❌ Wave 0 (small) |
| IDX-01 | existing indexer tests still pass | unit | `cargo test indexer` | ✅ `backend/src/indexer/tests.rs` |
| IDX-02 | skill versions flow forge-independent | unit/grep | `rg -n "github\.com" backend/src/server/*skill* ` / existing `server/tests.rs` skill tests | ✅ |
| UI-01 | `forgeTerms("github")` snapshot equals current strings; gitlab gives `!n`, "merge request" | vitest | `pnpm vitest run src/platform/lib/forgeTerms.test.ts` | ❌ Wave 0 |
| UI-01 | GitLab render shows no GitHubMark, link = backend URL | vitest component | `pnpm vitest run src/platform/pages/reviews` | ❌ (add to existing ReviewDetail/Reviews test) |
| UI-02 | changelog fragment valid; docs file exists | script | `pnpm changelog:check`; `test -f docs/gitlab.md` | ❌ Wave 0 |

### Sampling
- Per task: quick commands above. Per wave: full suite. Gate: full suite green (excluding the 20 baseline localStorage failures).

### Wave 0 Gaps
- [ ] `src/platform/lib/forgeTerms.test.ts`
- [ ] small pure `clone_url` helper + test in `backend/src/server/indexing.rs` tests (file `server/tests.rs` or new `indexing_tests.rs`)

## Security Domain
| ASVS | Applies | Control |
|---|---|---|
| V5 Input validation | yes | clone URL built from DB `full_name` as before; passed as git arg (no shell) |
| V7 Logging/secrets | yes | docs must not show tokens; error strings must not include PAT; `sanitize_url` precedent in `dependencies.rs` |
Threat: token in clone URL/logs (Information disclosure) — auth via `apply_git_auth` (not in URL); keep it.

## Sources
- Codebase (HIGH): `backend/src/server/mod.rs:288-295`, `server/indexing.rs:330-470`, `forge/mod.rs`, `gitlab/projects.rs`, `review/frozen.rs:58`, `server/routes.rs:24,57-108,194`, `src/types/settings.ts:140`, `src/platform/pages/ReviewsPage.tsx:650-660`, `changelog.d/feat-gitlab-connection.md`.

## Metadata
Confidence: stack HIGH, architecture HIGH, pitfalls MEDIUM (A1/A2 unverified).
**Research date:** 2026-11-05

## Orchestrator resolution of assumptions

- A1 resolved: `backend/src/gitlab/ca.rs:48` exports `GIT_SSL_CAINFO` (plus `SSL_CERT_FILE`, `GLAB_CA_CERT`) process-wide at boot, so git clones/fetches spawned by the indexer inherit the internal CA.
- A2 resolved: `review_archive/retention.rs:452` `split_once('/')` sits in `query_github_batch` (GitHub GraphQL merge backfill), which Phase 3 skips on GitLab. Not on the GitLab path.
- Spike script exists at `docker/gitlab-spike.py`.
