# Architecture Patterns

**Domain:** Code-hosting provider layer (GitHub | GitLab) inside a synchronous Rust review app
**Researched:** 2026-11-05
**Confidence:** HIGH on what the code does today (read directly). MEDIUM/LOW on GitLab/`glab` API behavior (training knowledge, not verified against docs; each item is flagged).

## Recommendation in one paragraph

Add a `forge` module with one object-safe `trait Forge: Send + Sync`, chosen once at startup (`OnceLock<Box<dyn Forge>>`, read from config) and used as `forge::current().method(...)`. Keep `backend/src/github/` as the GitHub implementation (do not move or rewrite it). Add `backend/src/gitlab/` as the second implementation. Neutral types and helpers that are not forge-specific today (diff parser, dedupe, payload files, `LivePrState`, `ReviewRequestSnapshot`, `StatusCommentApi`) move to `forge/`. Trait over enum: two impls chosen at runtime, tests can inject a fake, and the repo already uses this exact pattern (`StatusCommentApi`). No DB migration is needed for v1. Do the refactor with GitHub only first (zero behavior change), then add GitLab behind it.

## Where the seam goes

Today everything forge-specific funnels through `crate::github::*` (85 references in 27 files). The seam is that module boundary: callers outside stop naming `github::` and call `forge::current()`. Nothing deeper (runner, sandbox, review pipeline internals, DB) learns about forges, except the handful of hard-coded strings listed below.

```
 server/polling*.rs   server/publication.rs   server/review_*.rs   server/summary.rs
 server/status_comments.rs  queue_acknowledgements.rs  runtime.rs  routes.rs
 review/execution.rs  review/git.rs  review/frozen.rs  review/dependencies.rs
 review_archive/{mod,retention}.rs  indexer/mod.rs  server/indexing.rs  db/review_requests.rs
                          |
                          v   forge::current()  (&'static dyn Forge)
                 +-------------------+
                 |  forge/ (neutral) |  types, trait, diff parser, dedupe, payload files, CLI capture
                 +---------+---------+
                           |
            +--------------+---------------+
            v                              v
   github/ (gh, existing)          gitlab/ (glab, new)
```

Runners, the egress proxy and the DB never see the forge. Fetch and publication stay in `server`, as today, so the sandbox invariants are untouched. Egress allow-list does not change (GitLab never enters it).

## Component boundaries

| Component | Responsibility | Talks to |
|-----------|----------------|----------|
| `forge/mod.rs` (new) | `Forge` trait, `current()`, config selection (`OCR_FORGE` = `github` default or `gitlab`), neutral host validation, startup guard | all callers, both impls |
| `forge/types.rs` (new) | `ForgePr` (was `GhPullRequest`), `LivePrState`, `ReviewRequestSnapshot`, `IssueCommentUpsert`, `PreparedReview`, `ExistingOcrComment`, `DedupeResult`, `ForgeAuthStatus` | callers, impls |
| `forge/diff.rs` (new, moved from `github/pr.rs`) | unified-diff anchor parser (`diff_anchors`), `pr_file_paths` derived from it | impls supply diff text |
| `forge/dedupe.rs` (moved from `github/comments.rs`) | fingerprint, `dedupe_against`, `is_ocr_comment`, `with_fingerprint_marker` (all pure) | publication |
| `forge/transport.rs` (moved from `github/transport.rs`) | generalized `cli_capture_cancellable(program, env, args, cancel)`: temp-file capture, timeout, size cap, `kill_tree` | impls, `review_archive/retention.rs`, `review/dependencies.rs` |
| `github/` (existing) | `impl Forge for GitHub`, wraps current functions unchanged | `gh`, git |
| `gitlab/` (new) | `impl Forge for GitLab` using `glab api` | `glab`, git |
| `review/git.rs`, `server/summary.rs` | keep owning `git fetch`; take refspec and auth from forge | `forge` |

## The trait (smallest set that covers every call site)

Methods map one-to-one onto current `github::` functions; defaults make GitLab v1 able to skip hard features.

| Group | Methods |
|-------|---------|
| Identity | `id()`, `host()`, `clone_url(full_name)`, `commit_url(full_name, sha)`, `review_url(full_name, number, review_id)`, `pr_label()` (`"pull request"`/`"merge request"`), `pr_ref(number)` (`#12`/`!12`) |
| Git transport | `pr_head_ref(number)` (`refs/pull/N/head` vs `refs/merge-requests/N/head`), `apply_git_auth(&mut Command)` |
| Auth | `auth_status()`, `current_login()`, `sign_out()`, `run_auth_flow(...)` |
| Discovery | `list_workspaces()`, `list_repos(workspace)`, `list_open_prs(full_name)`, `list_open_prs_for_login(full_name, login)`, `live_pr_state[_cancellable]`, `pr_description`, `pr_diff(full_name, number) -> String` |
| Terminal-state sweep | `pr_terminal_states(batch)`; default impl loops `live_pr_state`; GitHub overrides with the existing GraphQL batch |
| Publication | `post_review(full_name, number, prepared)`, `find_review_id_by_marker`, `upsert_summary_comment`, `status_comments() -> Box<dyn StatusCommentApi>`, `existing_ocr_comments(full_name, number)` |
| Review requests | `snapshot_review_requests`, `restore_review_requests`; default no-op (GitLab) |
| Optional features | `supports_comment_activation()` (default false), `conversation_comments_page`, `activation_pr`, `unedited_author_command`; `supports_external_dependencies()` (default false) |

Free-function facade is not needed: sed `github::foo(` to `forge::current().foo(` at each site.

## Provider-neutral types: decisions

| Concept | Neutral form | GitHub | GitLab | Notes |
|---------|--------------|--------|--------|-------|
| PR number | `number: i64` (keep) | PR number | MR `iid` (not global `id`) | `UNIQUE(repo_id, number)` already right. Never expose GitLab `id`. |
| Repo identity | `full_name: String` | `owner/name` | `group[/sub...]/project` (project path with namespace) | Used as GitLab project id after URL-encoding `/` as `%2F` in `glab api projects/<enc>/...`. |
| `RepoInput.owner/name` | `owner` = everything before last `/`, `name` = last segment | unchanged | nested groups OK | Must change `split_once('/')` call sites (see assumptions below). |
| Workspace | `login` = group full path, or personal namespace | user/org | group/subgroup, user namespace | `kind`: add `"group"`; `workspaces.provider` column already exists. |
| Visibility | `is_private` | bool | `private`/`internal` => 1, `public` => 0 | |
| State | uppercase `OPEN`/`MERGED`/`CLOSED` | native | map `opened`/`merged`/`closed`/`locked` into those | Keeps all consumers (`classify_review_target`, polling) unchanged. |
| Live scope | `LivePrState {state, head_sha, base_ref, base_sha, merged_at}` | native | `sha`, `target_branch`, target tip, `merged_at` | `base_sha` must be the live tip of the target branch, not `diff_refs.start_sha` (can lag the target branch). Use a branch lookup (MEDIUM; verify). |
| Diff anchors | `HashMap<path, (RIGHT lines, LEFT lines)>` | parse `gh pr diff` | parse `glab mr diff --raw` | Parser is already git-style unified diff; reuse unchanged. Forge supplies text only. |
| Inline comment payload | keep GitHub-shaped JSON (`commit_id, body, comments[{path,line,side,start_line,start_side,body}]`) as the internal publish payload | posted as-is | GitLab impl translates | Avoids touching `publication.rs` payload building and tests. |
| Review identity | `Option<i64>` review id | review id | id of the marker note (see below) | |
| Status/summary comment | `StatusCommentApi` + `IssueCommentUpsert` | issue comments | MR notes | Skip `system` notes; author id from `author.id`. |
| `ReviewRequestSnapshot` | stays, JSON stored in DB | reviewers/teams | empty default | Posting a GitLab discussion does not consume a review request, so snapshot/restore are no-ops. |

### GitLab inline publication (the main semantic gap)

GitHub has a first-class atomic review object (one POST, whole 422 on a bad anchor, one id, dismissable). GitLab has none: each finding is a discussion (`POST projects/:id/merge_requests/:iid/discussions` with `position`: `position_type=text`, `base_sha`, `start_sha`, `head_sha`, `new_path`, `old_path`, `new_line` or `old_line`). Consequences for the design:

- Not atomic. Post inline discussions first, then post one top-level note carrying the review marker last. `find_review_id_by_marker` returns that note id, so "marker exists" means "publication completed". A retry after a partial failure is already made safe by the existing fingerprint dedupe (`dedupe_review_comments` runs before posting), so idempotency holds without new state.
- `ocr_review_id` / `github_review_id` / `github_comment_id` columns store this note id (all BIGINT; GitLab note ids are integers). Discussion ids are strings and are never needed by the product.
- Position needs `base_sha`/`start_sha`/`head_sha` of the MR version being commented (`diff_refs` of the MR). Publication already revalidates the live head before posting; the GitLab impl must use the diff_refs matching that head and fail (not fall back) if they do not match.
- `side` RIGHT => `new_line`; LEFT => `old_line`. Renames need `old_path`: get it from the diff text the impl already fetches for anchors.
- Multi-line findings (`start_line`) need `line_range` with `line_code` (path sha1 + old/new line). v1: anchor on the end line only; flag as a known loss. LOW confidence on exact `line_range` shape; research in the write-path phase.
- Nothing is approved automatically; no `event`/approve call is made on GitLab.

## Data flow

1. Startup: `forge::init()` reads `OCR_FORGE` (and `OCR_GH_HOST` or `OCR_GITLAB_HOST`), validates the host with the existing `is_valid_gh_host` (renamed neutral), writes `forge_provider`/`forge_host` into the existing `settings` table on first run and refuses to start if a later boot disagrees (enforces one provider per deploy without a migration).
2. Polling (`server/polling.rs`): `list_workspaces/list_repos` (Repository access) and `list_open_prs[_for_login]` -> `ForgePr` -> `polling_pr.rs` upsert into `pull_requests` (`number` = iid). New head => enqueue review as today.
3. Preparation (`review/execution.rs`): `live_pr_state_cancellable` -> `fetch_pr_head` using `forge.pr_head_ref(n)` into `refs/ocr/pr-N` -> `fetch_base` (already forge-neutral: `+refs/heads/<branch>`) -> frozen archive with `Scope.host = forge.host()`. Failed fetch still fails preparation.
4. Review: unchanged. Runner has IPC only.
5. Publication (`server/publication.rs`): `live_pr_state` guard -> `pr_diff` -> neutral anchor parser -> neutral dedupe against `existing_ocr_comments` -> `post_review` -> marker lookup/recovery in `runtime.rs` -> `upsert_summary_comment` and `status_comments()`. URLs built via `forge.commit_url` / `forge.review_url`.
6. Retention sweep (`review_archive/retention.rs`): `pr_terminal_states` -> merged/closed marks -> archive purge.

## Concrete list of call sites that must move behind the seam

Non-test call sites of `github::` outside `backend/src/github/` (from grep), grouped by what each needs:

| File | Uses today | Becomes |
|------|-----------|---------|
| `server/polling.rs` | `list_repos`, `current_login`, `list_open_prs`, `activation_pr`, `live_pr_state`, `list_open_prs_for_login`, `GhPullRequest` | trait methods; `activation_pr` gated by `supports_comment_activation()` |
| `server/polling_pr.rs` | `pr_file_paths`, `GhPullRequest`, `iso8601_to_epoch_ms`, `avatar_url` | `pr_diff` + neutral parser; `ForgePr.author.avatar_url` (GitLab returns it; GitHub impl fills the `.png` URL); neutral time parser must accept fractional seconds and offsets (GitLab) |
| `server/publication.rs` | `live_pr_state`, `GitHubStatusComments`, `snapshot_review_requests`, `prepare_review`, `post_review`, `find_review_id_by_marker`, `diff_anchors`, `dedupe_review_comments`, `DedupeResult`; hard-coded `https://github.com/{}/commit/{}` (lines ~120, 1152, 1229) and `/pull/{}#pullrequestreview-{id}` (~703) | trait methods; `commit_url`/`review_url` |
| `server/review_http.rs` | `current_login`, `snapshot_review_requests`, `prepare_review`, `post_review`, `find_review_id_by_marker`, `LivePrState`; hard-coded review URL (~204) | trait methods, `review_url` |
| `server/runtime.rs` | `GitHubStatusComments`, `find_review_id_by_marker`, `restore_review_requests`; hard-coded review URL (~335) | trait methods, `review_url` |
| `server/review_jobs.rs` | `live_pr_state_cancellable`, `pr_description` | trait methods |
| `server/summary.rs` | `upsert_issue_comment_by_marker`, `apply_git_https_auth`; hard-coded `refs/pull/{n}/head:refs/ocr/summary-{n}` (~249) | `upsert_summary_comment`, `apply_git_auth`, `pr_head_ref` |
| `server/status_comments.rs`, `queue_acknowledgements.rs` | `GitHubStatusComments`, `StatusCommentApi` | `forge::current().status_comments()`; trait moves to `forge/types.rs` |
| `server/comment_activation.rs` | `gh_host`, `ConversationComment`, `conversation_comments_page`, `unedited_author_command`; hard-coded commit URL | gated off for GitLab; `host()`/`commit_url`; GitHub-only wording stays for GitHub |
| `server/routes.rs` | `auth_status`, `sign_out`, `run_auth_flow`, `list_workspaces`, `list_repos`; routes `/auth/github/*`, `/github/workspaces` | trait methods; keep URLs for GitHub, add `forge` field (see frontend) |
| `server/workspace.rs` | `list_repos` | trait method |
| `server/indexing.rs` | `gh_host` for `https://{host}/{full_name}.git` | `clone_url` |
| `server/response.rs` | spawns `gh` with `apply_gh_host` to probe | forge-specific CLI probe (`glab auth status`) via trait or `auth_status` |
| `review/execution.rs` | `live_pr_state_cancellable`; message text "GitHub API" | trait method; neutral wording |
| `review/git.rs` | `apply_git_https_auth`; refspec `refs/pull/{n}/head` | `apply_git_auth`, `pr_head_ref` |
| `review/frozen.rs` | `gh_host` for `Scope.host` | `host()` |
| `review/dependencies.rs` | `gh_capture_cancellable` x2 (repo read access, public-repo checks) | gate external dependency discovery with `supports_external_dependencies()`; GitHub keeps current path |
| `review_archive/mod.rs` | `is_valid_gh_host`; `Scope::validate` requires exactly two path segments | neutral `forge::is_valid_host`; allow >= 2 segments |
| `review_archive/retention.rs` | GraphQL batch via `gh_capture_cancellable`, `iso8601_to_epoch_ms` | `pr_terminal_states` |
| `indexer/mod.rs` | `apply_git_https_auth` | `apply_git_auth` |
| `db/review_requests.rs` | `github::ReviewRequestSnapshot` | `forge::ReviewRequestSnapshot` (type move only) |
| Tests (`server/tests.rs`, `comment_activation_tests.rs`, `status_comments_test_support.rs`, `review/tests/usage.rs`) | `gh_host()`, fakes | follow the renames; no logic change |

Also forge-assumption sites that are not `github::` calls (found by grep): `sandbox/mod.rs:99` and `sandbox/egress.rs` tests (test fixtures only, no change), `github/` internals tied to `owner/name` (`split_once('/')` in `retention.rs`, `Scope::validate`, `RepoInput` construction in `github/repo.rs`).

## Assumptions that break for GitLab (must be fixed in the neutral refactor)

1. Exactly two path segments in `full_name` (`review_archive::Scope::validate`, retention `split_once`). GitLab nested groups need >= 2 segments. Relax in the GitHub-only phase with GitHub tests unchanged.
2. Hard-coded `https://github.com/...` URLs even with `OCR_GH_HOST` set (existing latent bug for GHE). Replace via `commit_url`/`review_url`; GitHub strings must stay byte-identical when host is `github.com`.
3. `iso8601_to_epoch_ms` reads only the first 19 chars (ignores fractional seconds and offset). Fine for GitHub `Z`, wrong for `+02:00` offsets. GitLab usually returns `...T10:00:00.123Z` (MEDIUM); make the parser offset-aware before GitLab.
4. Error classification by string: `comment_already_deleted` matches `(HTTP 404)`; `glab` error text differs (LOW). Put it per forge.
5. `gh_capture_cancellable` messages say "GitHub request ...". Parameterize the program and label.
6. Git auth: `apply_git_https_auth` builds `http.https://<host>/.extraheader` with `x-access-token:<token>`. GitLab expects Basic with username `oauth2` (OAuth) or any non-blank user with a PAT (MEDIUM). Token source is `glab`'s config, not `gh auth token` (LOW on the exact command; verify). Keep the token out of logs; add a redaction test (CONCERNS.md recommendation).
7. Self-signed/internal CA: `git` needs it via system trust store or `GIT_SSL_CAINFO`; `glab` (Go) honors system roots / `SSL_CERT_FILE`. Install the CA in the server image trust store rather than disabling verification.

## Database

No schema change needed for v1. Reasons:

- `workspaces.provider TEXT DEFAULT 'github'` exists; GitLab writes `'gitlab'` (a deploy never mixes, so it is informational).
- `repos.full_name` is `TEXT UNIQUE`, `owner`/`name` are text: nested paths fit.
- `pull_requests.number BIGINT` with `UNIQUE(repo_id, number)`: iid fits.
- `github_comment_id`, `github_review_id`, `github_review_url`, `github_updated_at`, `pr_summaries.github_comment_id` are misleading names but store BIGINT ids / URLs / epoch ms that GitLab can fill. Renaming would need a new migration and touches every query in `db/` with no functional gain. Do not rename in this milestone.
- Provider-switch protection lives in the existing `settings` key/value table (startup guard above), not in schema.

Only add a migration if a later phase proves a real need; append it at the end of `migrate.rs` and never edit applied ones.

## Frontend

About 83 non-test lines mention GitHub across `src/` (onboarding, access, settings, reviews, usage); `GitHubMark.tsx` and `settings/GitHubAuthRecovery.tsx` are GitHub-specific; services call `/auth/github/*` and `/github/workspaces`.

- Add `forge` info (`provider`, `host`, labels) to the existing status payload (`server/routes.rs` already returns `github: github_status()`); keep the route paths for now (renaming routes is churn with no user value).
- Put the terms in a new small file (e.g. `src/platform/lib/forge.ts`: `prLabel`, `prRef`, `prUrl`, mark component choice). Do not grow `ReviewsPage.tsx` (782) or `reviewFlow.ts` (779); both are near the 800 cap.
- Sign-in on GitLab: v1 is status-only plus actionable Docker guidance (`docker compose exec server glab auth login --hostname ...`); the device-flow UI in `run_auth_flow` is GitHub-specific and not worth porting first.
- Run Biome/Knip/`check:file-size` per phase.

## Patterns to follow

- Dispatch once: `forge::current()` returns `&'static dyn Forge`; init guarded by `OnceLock`, same style as the existing `GH_HOST` static.
- Normalize at the edge: GitLab impl returns GitHub-shaped state strings, payloads and ids, so the review/publication code does not branch on provider.
- Capability defaults instead of `if gitlab` in callers: `supports_*` flags and default no-op methods.
- Keep one HTTP/CLI capture function (temp files, 120 s timeout, 32 MB cap, `kill_tree`) for both CLIs.
- Fakes for tests: a `FakeForge` in `forge/` lets publication and polling tests run without `gh`/`glab`.

## Anti-patterns to avoid

- `if forge == "gitlab"` branches inside `publication.rs` (already 1444 lines; cap 2500) or the review pipeline.
- Enum dispatch with a `match` in every caller. The central match is the same size as the trait but loses fakes.
- Renaming DB columns or storing credentials in the DB.
- Moving or editing `backend/src/github/` internals during the same PR that introduces the trait (risk to the GitHub flow). Wrap, do not rewrite.
- Falling back to stale refs or the default branch when an MR fetch fails (violates review invariants).
- Giving runners any GitLab host access or token.

## Suggested build order (minimizes risk to GitHub)

1. **Neutral refactor, GitHub only (no behavior change).** Create `forge/` (trait, types, diff parser, dedupe, transport, URL helpers), add `impl Forge for GitHub` delegating to existing functions, rewrite the ~26 files' call sites mechanically, replace hard-coded URLs/refspec with trait calls, relax the two-segment `full_name` rule. Gate: `cargo test --workspace`, clippy, `pnpm test`, `docker/runner-smoke.py`; GitHub-posted bodies and URLs byte-identical. This is the only phase that touches the working product, so land it alone.
2. **Provider selection, image and auth.** `OCR_FORGE`, `OCR_GITLAB_HOST`, startup guard in `settings`, pinned and checksum-verified `glab` plus internal CA in the server image, `GitLab` stub, `auth_status` for GitLab. Gate: compose config and GitHub smoke unchanged.
3. **GitLab read path.** `list_workspaces/list_repos`, `list_open_prs[_for_login]`, `live_pr_state`, `pr_description`, `pr_diff`, `pr_head_ref`, git auth, offset-aware timestamp parsing, polling. Gate: a real MR is discovered and reviewed with fake inference, nothing posted.
4. **GitLab write path.** Status comments, summary note upsert, inline discussions with position, marker-last review identity, dedupe source, `pr_terminal_states`, URLs. Gate: idempotent re-publication test (partial failure then retry) against a real instance.
5. **Frontend terms and links, indexing and skills verification** for GitLab projects (clone URL is already host+path based).
6. **Deferred/optional:** `/ocr review` comment activation (GitLab has no repo-wide notes feed; skip), cross-repo dependency evidence (`supports_external_dependencies` false in v1), multi-line `line_range`, in-UI sign-in.

## Scalability considerations

| Concern | Single team (target) | Larger |
|---------|----------------------|--------|
| API calls per poll | one `glab api` list per selected project | add `updated_after` filter and pagination; watch rate limits |
| Terminal-state sweep | default loop, one call per tracked MR | add a project-level list by `state=merged` if slow |
| Instances | one per deploy by design | out of scope |

## Open questions needing phase-specific research

- Exact `glab api --paginate` output shape (concatenated arrays vs merged; GitHub code relies on `gh --slurp`, which `glab` likely lacks). LOW.
- `glab` auth token retrieval for git-over-HTTPS and `GITLAB_HOST` / config-dir env handling inside the container. LOW.
- `diff_refs` vs live target-branch tip for `base_sha`; behavior when the target moved but the MR diff has not refreshed. MEDIUM.
- `line_range` / `line_code` for multi-line comments; position errors on unchanged lines in large diffs (GitLab may collapse large diffs). LOW.
- `changes_count` can be a string like `"1000+"`; draft flag naming across GitLab versions. LOW.
- How `review/dependencies.rs` is entered, to place the `supports_external_dependencies()` gate cleanly (file read only partially). MEDIUM.

## Sources

- Code read directly: `backend/src/github/{mod,transport,repo,comments,pr,status_comments,review_requests,activation,auth}.rs`, `backend/src/review/{git,execution,frozen,dependencies}.rs`, `backend/src/review_archive/{mod,retention}.rs`, `backend/src/server/*` call sites, `backend/src/db/pg/0001_baseline.sql`, `.planning/PROJECT.md`, `.planning/codebase/CONCERNS.md`, `AGENTS.md` (HIGH).
- GitLab API/`glab` behavior: training knowledge only, not verified against docs in this pass (flagged LOW/MEDIUM above).
