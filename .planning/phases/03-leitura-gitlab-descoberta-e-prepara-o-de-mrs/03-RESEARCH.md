# Phase 3: Leitura GitLab: descoberta e preparação de MRs - Research

**Researched:** 2026-11-05
**Domain:** Rust backend (forge seam, `glab` CLI, git transport) + one React settings screen
**Confidence:** MEDIUM. Code touch points are HIGH (read in this session). GitLab REST field names and `glab api --paginate` output shape come from training knowledge and the unrun Phase 2 spike, so they are `[ASSUMED]` and every parser must tolerate variants.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- `full_name` de projeto GitLab = `path_with_namespace` (ex. `grupo/sub/proj`). Onde o código parte `owner/name`, usar `rsplit_once('/')`: owner = namespace completo, name = último segmento. Para GitHub (dois segmentos) o resultado é idêntico.
- Validações que exigem exatamente dois segmentos (`Scope::validate`, retenção) aceitam 2 ou mais segmentos não vazios.
- Número do PR = `iid` do MR (por projeto), mesma coluna/tipo do número do PR do GitHub. Sem migration se a coluna já for inteiro.
- Listagem: `glab api --paginate "projects?membership=true&min_access_level=30&archived=false&simple=true&per_page=100"`. Grupos/subgrupos vêm de `namespace.full_path`; a UI agrupa por namespace usando o seletor existente (`RepositorySelector`, workspaces = namespaces). Parser de paginação tolerante: aceitar arrays JSON concatenados e NDJSON.
- Polling: `glab api --paginate "projects/<id>/merge_requests?state=opened&per_page=100"`; head = `sha`; draft = `draft` (fallback `work_in_progress`) respeitando `skip_draft_prs` como `polling_pr.rs`. MRs que somem da lista de abertos saem da fila pelo mesmo caminho dos PRs fechados. Novo head enfileira e supersede reusando `review_jobs.rs`, sem ramificar por provider além do forge. 429/5xx: retry com backoff curto; erro persistente vira erro de polling existente, não panic.
- Preparação: fetch `refs/merge-requests/<iid>/head` do projeto alvo + target branch, sem checkout; fetch falho falha a preparação. SHA buscado deve bater com `sha` da API; divergência falha e o próximo polling tenta de novo. Base viva = tip da target branch; merge-base e diff locais (sem `/changes` nem `/diffs`).
- Git auth: extraheader `Authorization: Basic base64(oauth2:<token>)` via env `GIT_CONFIG_*`, token lido do config do glab (`glab auth status --show-token` ou `config.yml`), nunca em argv nem log.
- URLs vêm do forge: `https://<host>/<ns>/<proj>/-/merge_requests/<iid>` e `/-/commit/<sha>`.

### Claude's Discretion
Estrutura de `backend/src/gitlab/`, nomes, encoding do id do projeto (path URL-encoded ou id numérico). Velocidade acima de polimento; testes unitários com fixtures JSON para os parsers.

### Deferred Ideas (OUT OF SCOPE)
None. Nothing is posted to GitLab in this phase (Phase 4).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FORGE-03 | Nested `grupo/sub/proj` identity works end to end | `rsplit_once` sites (table below), `Scope::validate` `>= 2` segments, `repoOwner` TS, nested checkout dir `repos_dir/owner/name` already works |
| REPO-01 | Admin sees groups/subgroups/projects | New `gitlab/projects.rs` + forge dispatch of `list_workspaces`/`list_repos` + `refresh_github_workspaces` |
| REPO-02 | Open MRs listed by polling | `gitlab/mrs.rs::list_open_prs` -> `GhPullRequest`; remove `require_github` in `poll_reviews`; start scheduler for GitLab |
| REPO-03 | Draft respects `skip_draft_prs` | Map `draft`/`work_in_progress` to `is_draft`; `poll_one_pr` already applies `repo.skip_draft_prs` |
| REPO-04 | Closed/merged leave the queue | `forge::live_pr_state` GitLab impl feeds `reconcile_pr_lifecycle` unchanged |
| REPO-05 | New push enqueues new review and supersedes | Nothing to build: head change goes through `poll_scope_needs_reset` + `supersede_active_review`; only needs `sha` as `head_ref_oid` |
| REV-01 | Fetch `refs/merge-requests/<iid>/head`, no checkout, fail on fetch failure | `review/git.rs:21` already uses `forge::current().pr_head_ref`; needs GitLab git auth in `apply_git_https_auth` |
| REV-02 | Live base/head/merge-base | `live_pr_state` GitLab impl (MR GET + branch GET); rest is existing local git |
| REV-03 | Local diff, no truncation, fork MRs without fork creds | Existing flow is already local git; fork works because the ref lives in the target project |
</phase_requirements>

## Summary

Phase 1 left a seam: `backend/src/forge/mod.rs` does `pub(crate) use crate::github::*;` and every caller outside `github/` calls `forge::xxx`. A function **defined explicitly in `forge/mod.rs` shadows the glob re-export**, so the smallest change is to add thin dispatch wrappers in `forge/mod.rs` (GitHub arm calls `crate::github::xxx`, GitLab arm calls new `crate::gitlab::xxx`) and leave all `server/` call sites untouched. The GitLab arms return the **same Rust types** (`WorkspaceInput`, `RepoInput`, `GhPullRequest`, `LivePrState`), so `polling.rs`, `polling_pr.rs`, `review_jobs.rs` and `execution.rs` need almost no edits.

Most of REV-01..03 already works: the review flow fetches the PR ref with git, resolves the base from a fresh `origin/<branch>` fetch, and diffs locally. What blocks GitLab today is (a) the Phase 2 guards (`require_github`, `if github {…}` in `server/mod.rs`), (b) git auth, which is GitHub-token-only (`apply_git_https_auth`), (c) `forge::gh_host()` used for the clone URL and the archive `Scope.host`, (d) the `Scope::validate` two-segment check, and (e) GitHub-only helpers reached during preparation (`live_pr_state`, `pr_description`, `dependencies::capture`).

**Primary recommendation:** add `backend/src/gitlab/{api,projects,mrs,git_auth}.rs`, add dispatch wrappers in `forge/mod.rs`, flip the four guards, loosen the two-segment validations, and test every parser with JSON fixtures plus a fake `glab` shell script (pattern in `backend/src/gitlab/auth_tests.rs`).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Project/group listing | API / Backend (`gitlab/projects.rs` via `glab api`) | Browser (RepositorySelector) | Token stays server-side |
| MR discovery and lifecycle | API / Backend (polling thread) | Database (`pull_requests`) | Existing polling pipeline |
| Git fetch of MR ref, base, diff | API / Backend (server git subprocess) | — | Runners never get credentials; server prepares evidence |
| Git credential injection | API / Backend (env on child process) | — | Never argv, never log, never DB |
| Nested namespace display/matching | Browser (`repoOwner`) | API (owner/name split) | Both sides must agree on "owner = everything before last `/`" |
| Frozen evidence scope identity | API / Backend (`review_archive::Scope`) | — | host + repository must validate for 3+ segments |

## Standard Stack

No new crates and no new npm packages. Everything uses what is already in `backend/Cargo.toml`.

### Core
| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| `serde_json` (existing) | existing lock | `Deserializer::from_slice(..).into_iter::<Vec<T>>()` parses concatenated arrays and NDJSON with one call | Satisfies the "tolerant paginator" decision with no new code path |
| `glab` 1.128.0 (in server image since Phase 2) | pinned | `glab api [--paginate] <endpoint>` with `GITLAB_HOST`, `GLAB_CONFIG_DIR` | Locked by project constraint |
| `git` (existing) | image | fetch/clone/diff | Existing review pipeline |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `glab api` | `reqwest` + token from config.yml | Would put the token in Rust memory for REST; project constraint says `glab` only |
| Project path URL-encoded in endpoints | numeric project id | Id needs a stored column (migration); path encoding needs none. **Use URL-encoded `full_name`** (`/` -> `%2F`) |

**Installation:** none. **Package Legitimacy Audit:** no external packages are added, so slopcheck does not apply. `Packages removed: none`, `Packages flagged: none`.

## Architecture Patterns

### System Architecture Diagram

```
Settings UI (RepositorySelector)
   | GET /github/workspaces (route name kept)
   v
routes.rs refresh_github_workspaces --forge::list_workspaces()/list_repos()--+
                                                                              |
poll scheduler (server/mod.rs, now also for GitLab)                           v
   -> poll_reviews -> run_poll_reviews -> forge::list_open_prs(full_name) --> forge/mod.rs dispatch
        -> poll_one_pr (lane, draft, supersede)                                 |        |
        -> reconcile_pr_lifecycle -> forge::live_pr_state                       |        |
   -> start_review_job -> review_jobs.rs                                  GitHub: gh   GitLab: gitlab::{projects,mrs}
        -> ensure_repo_checkout (clone https://host/ns/proj.git, auth env)         |
        -> forge::live_pr_state_cancellable (MR GET + branch GET)                   v
        -> forge::pr_description (MR GET)                                    gitlab/api.rs run_api -> glab api (scrubbed env, RwLock read)
        -> review::execute -> fetch_pr_head (refs/merge-requests/N/head)
              -> resolve_base_rev (fetch refs/heads/<target>) -> local merge-base + diff
              -> frozen::capture (Scope{host=forge::gh_host(), repository=full_name})
   git child processes <- forge::apply_git_https_auth -> gitlab::git_auth (GIT_CONFIG_* env)
```

### Recommended Project Structure
```
backend/src/gitlab/
├── mod.rs        # add: mod api; mod projects; mod mrs; mod git_auth; re-exports
├── api.rs        # run_api(bin, host, dir, endpoint, paginate) -> Result<Vec<u8>>; retry; parse_pages<T>()
├── projects.rs   # GlProject, list_workspaces(), list_repos(login)
├── mrs.rs        # GlMr -> GhPullRequest; list_open_prs; live_pr_state; pr_description; current_login
├── git_auth.rs   # token_from_config(); apply(cmd, host)
└── api_tests.rs / projects_tests.rs / mrs_tests.rs  # fixtures + fake glab
```
`gitlab/auth.rs` is 365 lines; keep it. Reuse its private helpers by making `config_dir`, `apply_glab_env`, `run_glab`, `text_of`, `redact` `pub(super)`. Add a timeout parameter to `run_glab` (currently hard-coded 20 s at `auth.rs:209`; a paginated listing needs 120 s).

### Pattern 1: dispatch wrapper in `forge/mod.rs` (shadows the glob)
```rust
// forge/mod.rs: explicit items win over `pub(crate) use crate::github::*`
pub(crate) fn list_workspaces() -> Result<Vec<crate::db::WorkspaceInput>, String> {
    match current() { Forge::GitHub => crate::github::list_workspaces(), Forge::GitLab => crate::gitlab::list_workspaces() }
}
```
Wrap exactly these (GitHub arm delegates unchanged, so GitHub behavior is byte-identical):

| forge fn | Used at | GitLab behavior in Phase 3 |
|----------|---------|----------------------------|
| `list_workspaces`, `list_repos(login)` | `routes.rs:361-367`, `workspace.rs:13`, `polling.rs:110` | `gitlab::projects` |
| `list_open_prs(full_name)` | `polling.rs:146` | `gitlab::mrs` |
| `list_open_prs_for_login` | `polling.rs:301` | list open + filter by `assignees`/`reviewers` username (cheap) |
| `activation_pr` | `polling.rs:155,177` | `Ok(None)` (author commands are GitHub-only) |
| `live_pr_state`, `live_pr_state_cancellable` | `polling.rs:236`, `review_jobs.rs:136`, `execution.rs:22`, `publication.rs:41,729` | `gitlab::mrs::live_pr_state` |
| `pr_description` | `review_jobs.rs:165` | MR GET: `title`, `description` (null -> `""`) |
| `current_login` | `polling.rs:122`, `review_http.rs:73` | `glab api user` -> `username` |
| `gh_host` | `review/frozen.rs:37`, `server/indexing.rs:439` | `host()` (GitLab) / `github::gh_host()` (GitHub) |
| `apply_git_https_auth` | `review/git.rs:458`, `indexer/mod.rs:620` | `gitlab::git_auth::apply` |
| `avatar_url` | `polling_pr.rs:185` | `https://{host}/{login}.png` fallback only |
| `pr_file_paths` | `polling_pr.rs:12` (non-test) | not reachable: GitLab maps `changed_files = Some(0)` (below) |

Everything else stays `require_github`-guarded (comments, `post_review`, `diff_anchors`, status comments, review requests): Phase 4.

### Pattern 2: `glab api` call (no lock for reads)
Env is the Phase 2 `apply_glab_env` (sets `GITLAB_HOST`, `GLAB_CONFIG_DIR`, scrubs token vars). Change `GLAB_LOCK` from `Mutex` to `RwLock`: `login`/`logout` take write, `api` reads take read, so a 100-project polling pass does not block the Settings status call or serialize behind itself. `[ASSUMED]` glab reads `config.yml` at start only; the write/read race is the reason the lock exists (Phase 2 WR-05), so keep writers exclusive.

Endpoints (project id = URL-encoded `full_name`; branch name URL-encoded too):
```
projects?membership=true&min_access_level=30&archived=false&per_page=100      (paginate)
projects/<enc>/merge_requests?state=opened&per_page=100                       (paginate)
projects/<enc>/merge_requests/<iid>                                            (state, sha, target_branch, merged_at, title, description)
projects/<enc>/repository/branches/<enc-branch>                                (commit.id = live base tip)
user
```
**Deviation to flag:** CONTEXT lists `simple=true`. `[ASSUMED]` `simple=true` omits `visibility`, which UI-SPEC needs for Private/Public. Drop `simple=true` (payload is larger but `per_page=100` keeps it fine). If the spike shows `visibility` is present under `simple`, restore it.

### Pattern 3: tolerant paginator
```rust
// Accepts "[..][..]" (glab --paginate raw concat), NDJSON, or one array.
fn parse_pages<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<Vec<T>, String> {
    let mut out = Vec::new();
    for page in serde_json::Deserializer::from_slice(bytes).into_iter::<Vec<T>>() {
        out.extend(page.map_err(|e| format!("Unexpected GitLab response: {e}"))?);
    }
    Ok(out)
}
```
Error text must not embed the payload (it could include private project names). Never log stdout.

### Pattern 4: MR JSON -> `GhPullRequest` (all fields are `pub`, build the struct directly)
| GhPullRequest | GitLab field |
|---------------|--------------|
| `number` | `iid` |
| `title` | `title` |
| `author.login` / `avatar_url` | `author.username` / `author.avatar_url` |
| `is_draft` | `draft.or(work_in_progress).unwrap_or(false)` |
| `head_ref_oid` | `sha` |
| `base_ref_name` | `target_branch` |
| `head_ref_name` | `source_branch` |
| `base_ref_oid` | `None` (live base is resolved at review time; see pitfall 6) |
| `url` | `web_url` |
| `updated_at` | `updated_at` (ISO with fractional seconds; `iso8601_to_epoch_ms` reads the first 19 chars and assumes UTC) |
| `changed_files` | `Some(0)` with `// shortcut: list has no file count, review-time check enforces the limit`. It must be `Some` because `polling_pr.rs:300,303` set eligibility only when `changed_files.is_some()`. The real limit is enforced at `db/review_runs.rs:587` with local counts |
| `assignees` / `review_requests` | `assignees[].username` / `reviewers[].username` as `login` |

`LivePrState.state` mapping: `opened`->`OPEN`, `closed`->`CLOSED`, `merged`->`MERGED`, `locked`->`OPEN`. The rest of the code compares `== "OPEN"` (`review_http.rs:291`, `execution.rs:23`) and `eq_ignore_ascii_case("MERGED")` (`polling.rs:251`). `merged_at` = `merged_at` via `iso8601_to_epoch_ms`.

### Pattern 5: git auth (REV-01)
Today GitHub does: `GIT_CONFIG_COUNT=1`, `GIT_CONFIG_KEY_0=http.https://<host>/.extraheader`, `GIT_CONFIG_VALUE_0=AUTHORIZATION: basic <b64(x-access-token:TOKEN)>` (`github/auth.rs:247-257`), token from `GH_TOKEN` env or `gh auth token` (cached in `GIT_TOKEN` OnceLock, `auth.rs:212`). The GitLab arm is the same env shape with `oauth2:<token>`:
```rust
// gitlab/git_auth.rs
pub(crate) fn apply(cmd: &mut Command, host: &str) {
    if let Some(token) = token_from_config(host) {          // re-read each call, no OnceLock: reconnect must take effect
        cmd.env("GIT_CONFIG_COUNT", "1")
           .env("GIT_CONFIG_KEY_0", format!("http.https://{host}/.extraheader"))
           .env("GIT_CONFIG_VALUE_0", format!("AUTHORIZATION: basic {}", crate::github::basic_auth_for("oauth2", &token)))
           .env("GIT_TERMINAL_PROMPT", "0");
    }
}
```
- Generalize `github::basic_auth_token` (hard-codes `x-access-token:`, `auth.rs:260`) into `basic_auth_for(user, token)` and keep `basic_auth_token` as a one-line wrapper (tests may reference it).
- `token_from_config`: read `<OCR_DATA_DIR>/glab/config.yml` (same dir as `auth.rs::config_dir`), find the `host:` block, take its `token:` line, strip quotes and a `!!null` style tag. `[ASSUMED]` layout `hosts:\n  <host>:\n    token: <value>` (spike item 10). Fallback if not found: `glab config get token --host <host>` `[ASSUMED]`. Do not use `glab auth status --show-token` (prints to stderr in a human format, fragile to parse). Never put the value in an error string; pass errors through `auth::redact`.
- CA: `GIT_SSL_CAINFO` is already set process-wide by `gitlab::ca::prepare` before threads start, so git children trust the internal CA with no extra work.
- All git subprocess builders already call `crate::forge::apply_git_https_auth` (`review/git.rs:458`, `indexer/mod.rs:620` via `apply_github_auth`), so one dispatch covers clone, fetch, base fetch and index git calls. The env is visible only to the git child, never argv.

### Anti-Patterns to Avoid
- **`split_once('/')` on a full name** for any GitLab-reachable path (see table below).
- **Branching on provider inside `server/`** beyond the four guards. Keep provider differences inside `forge/mod.rs` and `gitlab/`.
- **Caching the git token in a `OnceLock`** like GitHub does: a Reconnect in Settings would not take effect until restart.
- **Calling the GitLab `changes`/`diffs` API for the diff.** Locked: local git only.
- **Holding a `Database` across glab calls** (existing anti-pattern; listing runs before `with_db_raw`).

## Exact Touch Points

### Guards to flip (GitLab currently skips these)
| File:line | Now | Change |
|-----------|-----|--------|
| `backend/src/server/polling.rs:72` | `forge::require_github("Pull request polling")?` | delete the line |
| `backend/src/server/mod.rs:291-303` | `let github = current()==GitHub; if config.auto_reindex && github {reindex}`; `if github { start_poll_scheduler }` | drop the `github` condition for both (reindex only does git clone/fetch via forge-dispatched auth: no `gh` calls, verified by grep of `server/indexing.rs`) |
| `backend/src/server/routes.rs:360` | `forge::require_github("GitHub workspace refresh")?` | delete; add GitLab branch (below) |
| `backend/src/server/polling.rs:155-200` | author-command scan (`activation_pr`, `poll_author_commands` -> `forge::conversation_comments_page`, GitHub GraphQL) | wrap the `poll_author_commands` call in `if forge::current() == Forge::GitHub`; `activated_open_pr_numbers` is empty on GitLab |
| `backend/src/server/queue_acknowledgements.rs:98` | posts via `GitHubStatusComments` | `if forge::current() == Forge::GitLab { return; }` first line (no rows exist on GitLab; guard prevents `gh` calls) |
| `backend/src/server/runtime.rs:422` `start_review_request_restoration` | GitHub review-request restore | harmless (no rows); leave |
| `backend/src/review_archive/retention.rs:~462` `backfill_merges` | GitHub GraphQL for merge detection | return `Ok(())` early for GitLab; merge/close detection comes from `reconcile_pr_lifecycle` + GitLab `live_pr_state` (`merged_at` feeds `mark_pr_terminal_with_merged_at`) |
| `backend/src/review/frozen.rs:~60` `dependencies::capture(..., true, ..)` | linked-repo/submodule hydration via `gh api` | for GitLab skip the call (treat like `inputs.offline`), `// shortcut: pinned dependencies are GitHub-only, revisit with Phase 4+` |

`review/execution.rs:22` (post-review scope re-check) and `review_jobs.rs:136` already go through `forge::live_pr_state_cancellable`, so they work once the wrapper exists.

### Workspace/repo listing (REPO-01)
- `projects.rs`: parse `GlProject { id, name, path, path_with_namespace, description, default_branch, visibility, archived, empty_repo, namespace { full_path, kind } }`. Skip `empty_repo == true` and `archived == true`.
- Workspaces = distinct `namespace.full_path`; `kind` = `"org"` for `group`, `"user"` for `user` (GitHub vocabulary the DB/UI already use). Sort + dedup like `github/repo.rs:14-15`.
- `RepoInput { workspace_login: namespace.full_path, owner: namespace.full_path, name: path, full_name: path_with_namespace, description, is_private: visibility != "public", default_branch }`. Guard each segment of `full_name` (non-empty, not `.`/`..`) before returning, because `owner/name` becomes a filesystem path (`repos_dir/<owner>/<name>`, `indexing.rs:410`, `polling_pr.rs:166`).
- One full listing per refresh. `refresh_github_workspaces` spawns one `list_repos(login)` thread per workspace; for GitLab add a branch that calls `gitlab::list_all()` once, upserts workspaces, then calls `db.upsert_repos` **once per workspace group** (`db/repositories.rs:284` only prunes removed repos for the *last* workspace in the slice). `forge::list_repos(login)` for GitLab = `list_all()` filtered by `workspace_login == login` (used by single refresh and monitor-all sync).
- `db/repositories.rs:158` hard-codes `'github'` for `workspaces.provider` (free-text column, no CHECK, nothing reads it): change to `provider_name(current())` or leave; low value.
- Nested group edge: a project whose namespace is a subgroup becomes a workspace `g/sub` and is not listed under `g`. Matches UI-SPEC ("flat entries by full path").

### `split_once('/')` sites
| File:line | Action |
|-----------|--------|
| `db/repositories.rs:6` `ensure_temp_repo_for_index` | `rsplit_once('/')` |
| `github/repo.rs:32`, `github/review_requests.rs:75` | GitHub-only; leave |
| `review_archive/retention.rs:450` | GitHub GraphQL batch; leave (skipped for GitLab) |
| `bench/mod.rs:121` | bench is GitHub-only; leave |
| `review_archive/mod.rs:266-277` `Scope::validate` | `names.len() >= 2` instead of `!= 2`; keep per-segment checks (non-empty, not `.`/`..`, `[A-Za-z0-9-_.]`) and the 256-char cap. Add test with `g/sub/proj` and keep rejecting `a` and `a//b` |
| `src/platform/lib/utils.ts:3` `repoOwner` | `const i = repo.fullName.lastIndexOf("/"); return i > 0 ? repo.fullName.slice(0, i) : repo.owner;` identical for 2 segments |

### Preparation path (REV-01..03), what already works
- `review/git.rs:22` fetches `forge::current().pr_head_ref(number)` into `refs/ocr/pr-<n>`; failure returns `could not fetch PR branch via git` (fails preparation).
- `review/execution.rs:120-135`: fetched SHA must equal `expected_scope.head_sha` (from live API) else `"PR head changed before review preparation. Refresh the PR and retry."`. `review_http.rs:300-305` classifies that prefix as a target change, so a lagging MR ref (PITFALLS) fails safe and the next poll retries. This **already implements** the "divergência falha" decision; only add a test.
- Base: `resolve_base_rev` fetches `+refs/heads/<branch>:refs/remotes/origin/<branch>` and pins `origin/<branch>`. Works as-is for GitLab. Merge-base and diff are local git (`changed_files_local`, `git diff`). No GitHub API diff is involved in preparation, so large MRs are not truncated by an API cap.
- Fork MRs: `refs/merge-requests/<iid>/head` is in the **target** project, so the clone only ever talks to the target remote with our token; no fork remote or credential is added.
- Clone URL `https://{host}/{full_name}.git` (`indexing.rs:439`) works for nested paths once `forge::gh_host()` returns the GitLab host.
- Archive `Scope.host` (`frozen.rs:37`) and `review_archive/mod.rs:268` `is_valid_gh_host` accept `host:port`.
- URLs: `Forge::commit_url/review_url` already exist for GitLab with nested names (Phase 1/2 tests).

## Frontend (UI-SPEC)

Smallest change set (all files < 800 lines; `SettingsPage.tsx` is 534):
1. **Auth gate is the real blocker.** `setupApi.setupStatus()` (`src/services/setupApi.ts:24`) calls `/auth/github/status`; on a GitLab deploy `gh` is not logged in, so `loggedIn=false`, `nextStep=1`, `workspaces=[]`, and `SettingsPage.tsx:115,123,428` block repository access ("Reconnect GitHub"). Fix on the backend in `routes.rs::github_auth_status` (`:169`): when `forge::current()==GitLab`, return the existing `GitHubAuthStatus` JSON shape built from `crate::gitlab::status(&forge::host())` (`loggedIn`, `login`, `host`, `detail: "glab authenticated"`). One place, zero TS type churn.
2. `repoOwner` (above) so `locked`/monitor-all matching works for nested namespaces.
3. `RepositorySelector` strings: add one optional prop `forge?: "github" | "gitlab"` (passed from `SettingsPage` using the existing `gitlabForge`). With `gitlab`: placeholder "Search by group or project name…", helper "Select groups above to browse their projects.", loading "Loading projects…", empty copy per UI-SPEC. GitHub strings stay byte-identical (default branch of the ternaries).
4. `SettingsPage.tsx:417,434` copy ("Select organizations first…", "Select organizations") may be made forge-aware with the same `gitlabForge` flag; keep optional, UI-SPEC only mandates the listed strings.
5. Error strings (UI-SPEC "Could not load projects from {host}…", "GitLab rejected the stored token…"): map in `gitlab/api.rs` using the `Code`/`classify` table in `auth.rs` (`Unauthorized`/`Expired` -> token message, otherwise list-failed message). Never include stderr text.

Existing tests: `src/platform/pages/settings/` has `GitLabConnection.test.tsx`; add `RepositorySelector.test.tsx` (copy by forge, nested `fullName`) and a `utils` test for `repoOwner`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Paginated JSON of unknown shape | manual bracket scanning | `serde_json::Deserializer::into_iter::<Vec<T>>` | Handles concatenated arrays, NDJSON, whitespace |
| Base64 for the extraheader | new encoder | existing `basic_auth_token` (generalize username) | Already tested and in use |
| Token scrubbing in errors | new regexes | `gitlab::auth::redact` | Covers literal token and `glpat-` runs |
| glab process supervision | new spawner | `run_glab` (drains pipes, kills on timeout) + a timeout parameter | Phase 2 fixed pipe-deadlock bugs here |
| URL encoding of project path | external crate | `full_name.replace('/', "%2F")` (segments are `[A-Za-z0-9-_.]`, validated); encode branch names with a tiny helper that percent-encodes everything outside unreserved | Branch names may contain `/`, `#`, `?`, `+` |
| Draft/skip, supersede, lifecycle | GitLab-specific copies | existing `poll_one_pr`, `reset_polled_review`, `reconcile_pr_lifecycle` | Same invariants as GitHub |

## Common Pitfalls

### Pitfall 1: `simple=true` hides `visibility`
**What goes wrong:** every project shows Public. **Avoid:** omit `simple=true` (deviation noted). **Warning sign:** fixture without `visibility` still parses; default to private (`!= "public"`) so missing means private.

### Pitfall 2: `glab api --paginate` output shape unknown
**What goes wrong:** only the first page parsed or parse error. **Avoid:** `parse_pages` above plus a fixture test for each shape; `[ASSUMED]` raw concatenation `[...][...]`. Spike item 4 resolves it.

### Pitfall 3: 20 s glab timeout and global lock
**What goes wrong:** big listing times out; polling starves Settings status. **Avoid:** timeout parameter (120 s for `api`), `RwLock` readers.

### Pitfall 4: retry on 429/5xx
`glab` exits non-zero and prints the HTTP status in stderr (`[ASSUMED]`, e.g. "429", "502"). Retry up to 3 times with 1 s, 2 s, 4 s sleeps when stderr contains `429` or `50[0-4]`; honor the cancel flag if one is available. Persisting failure returns `Err(String)` which `run_poll_reviews` already collects into `errors` (`polling.rs:200`) and records via `record_poll_result`; no panic path.

### Pitfall 5: MR ref lag after push
`refs/merge-requests/N/head` can trail the API `sha` for a moment. Already fails safe (see Preparation). If the spike shows frequent lag, add a short bounded retry (2 x 2 s) inside the GitLab arm of `fetch_pr_head` keyed on `rev-parse` mismatch; not needed to meet the criteria.

### Pitfall 6: no `base_ref_oid` in the list
`incoming_scope` in `polling_pr.rs:151-159` is `None` without it, so `base_scope_changed` is always false on GitLab polling. A target-branch advance alone does not reset a review; `live_pr_state` at review time and the end-of-review `scope::equivalent` re-check (`execution.rs:21-27`) still use the live base. Accept for Phase 3. If the planner wants parity, fill `base_ref_oid` from one `repository/branches/<target>` call per distinct target branch per project per poll (cache in a local `HashMap`).

### Pitfall 7: user namespaces and `path` collisions
`repos.full_name` is the unique key (`ON CONFLICT(full_name)`); GitLab `path_with_namespace` is globally unique, fine. A project moved/renamed gets a new `full_name` and the old row is pruned by `upsert_repos` (per-workspace grouping needed, see above).

### Pitfall 8: `merged_at` / `updated_at` offsets
`iso8601_to_epoch_ms` ignores fractional seconds and any `+hh:mm` offset (treats as UTC). GitLab returns `…Z` in API JSON `[ASSUMED]`; if a version returns an offset, merged-at is off by hours: harmless for retention (hours) but add a fixture with `.123Z`.

### Pitfall 9: token read from config.yml
`has_stored_login` (`auth.rs:238`) already marks layout as an assumption. Make `token_from_config` return `None` (not panic) on any parse failure; the git fetch then fails with an auth error and the review fails visibly instead of hanging on a prompt (`GIT_TERMINAL_PROMPT=0`).

## Code Examples

### Fake `glab` seam (follow `auth_tests.rs::fake`)
Make the new functions `*_at(bin: &Path, host: &str, dir: &Path, ...)` and public wrappers pass `Path::new("glab")`, exactly like `status_at`/`login_at`.
```rust
// api_tests.rs (#[cfg(unix)]): script chooses the fixture by args
let script = format!(r#"#!/bin/sh
echo "$*" >> "{d}/args.log"
case "$*" in
api\ --paginate\ projects?membership*) cat "{d}/projects.json";;
api\ --paginate\ projects/g%2Fsub%2Fp/merge_requests?state=opened*) cat "{d}/mrs.json";;
api\ projects/g%2Fsub%2Fp/merge_requests/7) cat "{d}/mr7.json";;
api\ projects/g%2Fsub%2Fp/repository/branches/main) echo '{{"commit":{{"id":"{base}"}}}}';;
*429*) echo "429 Too Many Requests" >&2; exit 1;;
esac"#, d = dir.display(), base = "b".repeat(40));
```
Assertions to include: args contain `membership=true&min_access_level=30`; no `glpat-` in `args.log`/errors; env has `GITLAB_HOST` and no token vars (reuse `apply_glab_env`); 429 twice then success retries and returns data; persistent 502 returns `Err` without payload text.

### Fixtures to write (JSON, in `backend/src/gitlab/fixtures/` or inline `r#""#`)
projects: user namespace project, group project, 3-level `g/sub/proj`, archived, empty_repo, missing `visibility`; MRs: draft via `draft`, draft via `work_in_progress` only, fork MR (`source_project_id != target_project_id`), `reviewers`, null `description`; MR GET for `opened/closed/merged/locked`.

### Git auth test (no network)
`git_auth::apply` on a `Command`, then `cmd.get_envs()` assert: `GIT_CONFIG_KEY_0 == http.https://gl.example.com/.extraheader`, value starts with `AUTHORIZATION: basic ` and decodes to `oauth2:<token>`, and `cmd.get_args()` is empty of the token. Token parse test on a sample `config.yml` string (quoted, unquoted, other host first).

## State of the Art

| Old | Current | Impact |
|-----|---------|--------|
| `work_in_progress` | `draft` (GitLab 14+) | read `draft` first, fall back `[ASSUMED]` |
| Per-MR `/changes` for diffs | local git diff | locked decision; avoids API diff truncation |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `projects?simple=true` omits `visibility` | Pattern 2 | Low: we omit `simple` anyway; default private |
| A2 | `glab api --paginate` prints concatenated arrays (or NDJSON) | Pattern 3 | Parser tolerates both; a single merged array also parses |
| A3 | `config.yml` layout `hosts:/<host>:/token:` with plain token under `--insecure-storage` | Pattern 5 | Git auth fails; fallback `glab config get token --host` |
| A4 | glab prints HTTP status (`429`, `5xx`) in stderr on failure | Pitfall 4 | Retry never triggers; polling just reports an error |
| A5 | `draft` and `work_in_progress` both present on MR list; `reviewers` present | Pattern 4 | Draft MRs reviewed despite skip setting if neither key exists |
| A6 | `oauth2:<PAT>` Basic header is accepted by GitLab git HTTP | Pattern 5 | Fetch 401; try `PRIVATE-TOKEN` style is not valid for git, change username only |
| A7 | API timestamps end in `Z` | Pitfall 8 | Minor time skew |
| A8 | GitLab >= 16 (floor from Phase 2) serves `refs/merge-requests/<iid>/head` for fork MRs in the target project | Preparation | REV-01/REV-03 fail; spike item 5 |
| A9 | glab reads config.yml only at process start | Pattern 2 | RwLock read could race a login; keep writers exclusive |

All A-items should be confirmed by the pending Phase 2 spike (`02-SPIKE.md`) once a real instance is available; none changes the structure of the plan.

## Open Questions

1. **Does a lagging MR ref occur often enough to need a retry?** Fails safe today; decide after the spike. Recommendation: ship without retry.
2. **`changed_files = Some(0)` vs fetching `changes_count`.** Recommendation: ship `Some(0)`; the file-limit lane is enforced at review time (`review_runs.rs:587`). The "too many files" skip lane will therefore appear later than on GitHub.
3. **Pinned dependencies (submodules) on GitLab.** Skipped in Phase 3 (documented shortcut). MRs touching submodules still review the parent repo only.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Real GitLab instance | live validation | no | n/a | JSON fixtures + fake glab script (this plan) |
| `glab` in server image | runtime | yes (Dockerfile, 1.128.0) | 1.128.0 | fake script in tests |
| Docker + WSL test harness | `cargo test` with Postgres | yes (`~/rusttest.sh`) | rust 1.98.1 | none needed |
| Node/pnpm | frontend tests | yes via `npx -y pnpm@10.15.0` | 10.15.0 | none needed |

Live GitLab behavior (pagination shape, config.yml, 429 strings, ref lag) stays unverified; carry it as human UAT (like Phase 2's `02-HUMAN-UAT.md`).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (needs `OCR_TEST_DATABASE_URL`, supplied by `~/rusttest.sh`); Vitest 4.1 + jsdom |
| Config file | `backend/Cargo.toml`; `vite.config.ts` |
| Quick run command | `wsl -e bash -lc '~/rusttest.sh "--lib gitlab"'` |
| Full suite command | `wsl -e bash -lc '~/rusttest.sh ""'` and `npx -y pnpm@10.15.0 test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FORGE-03 | `Scope::validate` accepts `g/sub/proj`, rejects `a`, `a//b`, `g/../x`; `rsplit_once` owner/name; `repoOwner` nested | unit | `~/rusttest.sh "--lib review_archive"`; `~/rusttest.sh "--lib gitlab"`; `pnpm test utils` | modify existing `review_archive/tests.rs`; Wave 0 for `gitlab/projects_tests.rs`, `src/platform/lib/utils.test.ts` |
| REPO-01 | project fixtures -> workspaces (kind, dedup) and `RepoInput` (private/public, skip archived/empty, nested) | unit | `~/rusttest.sh "--lib gitlab::projects"` | Wave 0 |
| REPO-01 | paginated shapes (concatenated, NDJSON, single array) | unit | `~/rusttest.sh "--lib gitlab::api"` | Wave 0 |
| REPO-02 | MR fixture -> `GhPullRequest` mapping, fake-glab list call args | unit (fake glab) | `~/rusttest.sh "--lib gitlab::mrs"` | Wave 0 |
| REPO-03 | `draft`/`work_in_progress` -> `is_draft`; lane via `poll_one_pr` with `skip_draft_prs` | unit | `~/rusttest.sh "--lib gitlab::mrs"`; existing `server::tests` draft cases | Wave 0 |
| REPO-04 | state mapping opened/closed/merged/locked + `merged_at`; lifecycle marks terminal | unit | `~/rusttest.sh "--lib gitlab::mrs"` | Wave 0 |
| REPO-05 | head change supersedes (existing `poll_scope_needs_reset`) with GitLab-shaped PR | unit | `~/rusttest.sh "--lib server::"` | exists (extend) |
| REV-01 | `apply` sets extraheader env only (no args), token parse from config.yml, fetch failure message has no secret | unit | `~/rusttest.sh "--lib gitlab::git_auth"` | Wave 0 |
| REV-02 | live state = MR GET + branch GET via fake glab; base/head SHAs | unit (fake glab) | `~/rusttest.sh "--lib gitlab::mrs"` | Wave 0 |
| REV-03 | local-git: temp repos with a `refs/merge-requests/7/head` fixture ref; `fetch_pr_head` with `Forge::GitLab` refspec against a local bare repo (file remote), SHA mismatch -> "PR head changed" | integration (local git) | `~/rusttest.sh "--lib review::"` | extend existing `review/tests` |
| UI | selector copy by forge, nested fullName filter | component | `npx -y pnpm@10.15.0 test RepositorySelector` | Wave 0 |

Note: `forge::current()` is a process `OnceLock` (defaults GitHub in tests). Tests must call host-explicit/`_at` functions and never rely on setting the forge global (Phase 2 WR-04 lesson); test the wrappers' GitLab arm through the `gitlab::` functions directly.

### Sampling Rate
- Per task commit: `wsl -e bash -lc '~/rusttest.sh "--lib gitlab"'`
- Per wave merge: full `~/rusttest.sh ""` plus `npx -y pnpm@10.15.0 test` and `pnpm check:file-size`
- Phase gate: full suite green; GitHub tests unchanged and passing (zero-regression gate); `cargo clippy -D warnings` and `cargo fmt --check` as the repo does

### Wave 0 Gaps
- [ ] `backend/src/gitlab/{api,projects,mrs,git_auth}.rs` and `*_tests.rs` with fixtures
- [ ] `src/platform/lib/utils.test.ts`, `src/platform/pages/settings/RepositorySelector.test.tsx`
- [ ] Extend `review_archive/tests.rs` for three-segment scope

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | token only in glab `config.yml` (0600 dir), read at call time, never cached or persisted by the app |
| V4 Access Control | yes | `min_access_level=30` limits listing; routes already behind `require_auth`/trusted origin |
| V5 Input Validation | yes | validate every path segment from GitLab before using it as a filesystem path and URL part; branch names URL-encoded |
| V6 Cryptography | no | TLS via system + internal CA bundle (Phase 2) |
| V7 Error handling/logging | yes | errors from classifiers only; no stderr/stdout pass-through; `redact` on any glab text |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Token in argv (`ps`) or logs | Information disclosure | env-only `GIT_CONFIG_*` on the git child; `glab api` never given the token; `eprintln!` never prints stdout |
| Path traversal via `path_with_namespace` into `repos_dir` | Tampering | segment validation (`.`/`..`/empty/charset) in `projects.rs`, plus `Scope::validate` |
| Fork MR supplying credentials or remotes | Elevation | only the target project's remote is used; no `git remote add` for forks |
| Runners receiving host credentials | Information disclosure | git auth env applied only in server-side git builders; `GIT_CONFIG_*` is not in the runner IPC; keep `docker/test-gitlab-isolation.py` passing |
| Secrets in persisted error text (review history) | Information disclosure | `classify_gh_error`-style fixed strings for the GitLab arms |
| Malicious branch name in URL path | Tampering | percent-encode before building endpoints; never shell-interpolate (args are passed as an array to `Command`) |

## Sources

### Primary (HIGH confidence)
- Codebase read this session: `backend/src/forge/mod.rs`, `forge/guard.rs`, `gitlab/{mod,auth,auth_tests,ca}.rs`, `github/{mod,repo,pr,transport,auth}.rs`, `server/{mod,polling,polling_pr,review_jobs,routes,workspace,indexing}.rs`, `review/{git,execution,frozen}.rs`, `review_archive/{mod,retention}.rs`, `db/repositories.rs`, `src/platform/pages/settings/RepositorySelector.tsx`, `SettingsPage.tsx`, `src/services/setupApi.ts`, `src/platform/lib/utils.ts`
- Phase docs: `03-CONTEXT.md`, `03-UI-SPEC.md`, `02-SPIKE.md`, `02-05-SUMMARY.md`

### Secondary / Tertiary (LOW, training knowledge, not re-verified)
- GitLab REST field names (`iid`, `sha`, `draft`, `work_in_progress`, `target_branch`, `reviewers`, `merged_at`, `namespace.full_path`, `visibility`), `simple=true` field set, `refs/merge-requests/<iid>/head`, `glab api --paginate` output, glab config.yml layout. Confirm against the target GitLab (`/api/v4/version`) and the Phase 2 spike.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH (no new dependencies)
- Architecture/touch points: HIGH (read from source)
- GitLab API/glab behavior: MEDIUM-LOW (assumptions A1..A9)
- Pitfalls: MEDIUM

**Research date:** 2026-11-05
**Valid until:** 30 days, or until the Phase 2 spike is run (then re-check A1-A9)
