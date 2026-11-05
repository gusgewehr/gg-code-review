# Domain Pitfalls

**Domain:** Self-hosted GitLab MR review support added to a GitHub-based review bot (OpenCodeReview, `gh` -> `glab`)
**Researched:** 2026-11-05
**Overall confidence:** MEDIUM. The GitLab API behaviours below come from my knowledge of the GitLab REST API (Discussions, Merge Request Versions, Diffs). I did not re-verify them against live docs in this pass, so confirm each against the docs for the target GitLab version (`GET /api/v4/version`) in the phase that builds it. Items marked (verify) are the least certain. Codebase facts come from `AGENTS.md`, `PROJECT.md` and `CONCERNS.md`.

## Critical Pitfalls

### 1. Inline position: wrong SHAs (base/start/head)
**What goes wrong:** `POST /projects/:id/merge_requests/:iid/discussions` with `position[position_type]=text` returns `400 ... position is invalid` or silently creates a non-inline (diff-less) note.
**Why it happens:** GitHub takes a `commit_id` plus path/line/side. GitLab needs all three of `base_sha`, `start_sha` and `head_sha`, and they must be the SHAs of one MR *version*. Common errors: using the target-branch tip as `base_sha`; using the merge-base for `start_sha`; taking `diff_refs` from a stale MR fetch; computing SHAs locally from `git merge-base` (which differs from GitLab's). For a normal "merge result" MR, GitLab's `start_sha` is the target-branch tip at the time of the version, and `base_sha` is the merge-base.
**Consequences:** Every finding fails to publish, or lands as a general note without a diff anchor, which breaks the "inline findings" core value.
**Prevention:**
- Take all three SHAs verbatim from `GET .../merge_requests/:iid/versions` (latest entry: `base_commit_sha`, `start_commit_sha`, `head_commit_sha`) or from `diff_refs` on the MR object. Never derive them from local git.
- Bind the published selection to the version it was validated against. The review's frozen head must equal the version's `head_commit_sha`; if they differ at publish time, treat it as a head change (re-review or skip), not a retry.
- Fetch `versions` immediately before publishing, not at review start.
**Detection:** a 400 on the first discussion of a smoke MR; discussions that come back with `position: null`, or `type: null` instead of `DiffNote`. Assert `type == "DiffNote"` in the response.
**Phase:** Publication phase. Smoke-test against a real instance first.

### 2. old_line / new_line rules
**What goes wrong:** the finding anchors to a changed line, but GitLab rejects it or shows it on the wrong side.
**Why it happens:** GitLab does not have GitHub's `side=LEFT|RIGHT`. The rules are:
- Added line: `new_line` only (no `old_line`).
- Removed line: `old_line` only (no `new_line`).
- Unchanged context line: both `old_line` and `new_line`, and they must be correct for that diff. Sending only one is a 400 or lands wrong.
- Both `new_path` and `old_path` are required (equal unless renamed).
AGENTS.md anchors findings to changed lines by `path`, line and side from the head or base snapshot. A direct port that sends `new_line = line` for a base-side finding anchors to the wrong line.
**Consequences:** off-by-N comments; "line code" errors; findings on removed lines placed on unrelated added lines.
**Prevention:** build the map (path, side, line) -> (`old_line`, `new_line`) from the parsed MR diff, the same diff the validator used (`PR_DIFF.patch` equivalent), not from file line numbers. One function with table-driven tests: added, removed, context, first/last hunk line, file with `\ No newline at end of file`, CRLF.
**Detection:** unit tests for the three line kinds; a smoke MR with one of each.
**Phase:** Provider abstraction (define a neutral "diff anchor" type) and Publication.

### 3. line_range / multi-line comments
**What goes wrong:** copying GitHub's `start_line`/`line` into GitLab's `position[line_range][start|end]` fails with 400.
**Why it happens:** `line_range` needs `line_code` (`<sha1(path)>_<old>_<new>`) and `type` (`new`/`old`) for both start and end, and it is only supported when the discussion position carries compatible old/new lines (verify against the target version; multi-line diff notes were added around 13.x).
**Prevention:** v1 posts single-line discussions only, anchored to the first changed line of the finding. Put multi-line spans in the body text. Treat `line_range` as a later enhancement.
**Phase:** Publication (decision), defer the feature.

### 4. Outdated positions after new pushes / force-push
**What goes wrong:** a new head arrives while publication or re-review is running; discussions get created against an old version, show as "outdated", or an update 400s. After a force-push, old discussions anchor to commits no longer in the MR.
**Why it happens:** GitLab positions are per version. On a new push GitLab tries to re-map discussions and marks those it cannot as outdated/resolved by change. Idempotency keyed only on the finding fingerprint will believe an outdated discussion is still the "existing" one and skip re-posting.
**Consequences:** duplicate comments or lost findings; stale discussions presented as live; publication bound to a head that is no longer the MR head.
**Prevention:**
- Keep the existing invariant: publication is bound to the exact validated selection and head SHA. Before posting, re-read the MR head; mismatch means abort and let polling enqueue the new head.
- Idempotency: identify existing OCR discussions by the `<!-- ocr-... -->` marker in the first note body, and also compare `position.head_sha` to the current version. A marker match on an outdated position is "stale", not "already posted" (decide: leave and post new, or resolve old).
- Do not auto-resolve human discussions; only touch notes carrying the OCR marker and authored by the bot user.
**Detection:** test with push-after-review and `git push --force`; check `position.head_sha` of listed discussions.
**Phase:** Publication; polling head-change logic.

### 5. Refs for fetching MR heads, and forked MRs
**What goes wrong:** copying `refs/pull/{n}/head` logic (`backend/src/review/git.rs`) to `refs/merge-requests/{iid}/head` works, then breaks on forks, deleted source branches or merged/closed MRs.
**Why it happens:**
- `refs/merge-requests/<iid>/head` exists in the *target* project for fork MRs too (GitLab creates it), so prefer it over the source branch name. Never fetch `source_branch` from `source_project_id`; that project may be private to the bot.
- `refs/merge-requests/<iid>/merge` only exists for mergeable MRs and is regenerated asynchronously; do not use it as the head.
- The refs can lag the API for a moment after a push; `head_sha` from the API may not yet be fetchable (race). Pruned/GC'd refs for old closed MRs (verify).
- Refspec specifics: `+refs/merge-requests/N/head:refs/ocr/mr/N/head` style; keep to a private namespace, never touch local branches.
**Prevention:** fetch the ref, then verify the fetched OID equals the API `diff_refs.head_sha`; retry a bounded number of times, then fail preparation (AGENTS.md: a failed fetch fails preparation, never fall back to stale refs). Fetch the target branch tip live too for base and merge-base; use the `versions` `base_commit_sha` only to cross-check.
**Detection:** a fork MR in the smoke set; test where the API head is ahead of the fetched ref.
**Phase:** Review preparation (git layer) abstraction.

### 6. Internal CA / self-signed TLS (glab, git, server HTTP)
**What goes wrong:** `x509: certificate signed by unknown authority` from `glab`; `SSL certificate problem` from `git fetch`; and the server's own HTTP/`curl` checks fail. Developers "fix" it with `GIT_SSL_NO_VERIFY` or `glab ... --insecure` (or `skip_tls_verify`), a security regression.
**Why it happens:** three separate trust stores. `glab` (Go) uses the system roots (`SSL_CERT_FILE`/`SSL_CERT_DIR` honoured on Linux, so `update-ca-certificates` in the image or a mounted bundle works). `git` uses libcurl/OpenSSL with `http.sslCAInfo` or `GIT_SSL_CAINFO`. Rust code that calls HTTPS directly (if any) uses its own roots. A CA mounted only for one of them leaves the others failing. Also: bundle must include the *intermediate* chain if the server does not send it.
**Prevention:**
- One mounted CA bundle (e.g. `OCR_GITLAB_CA_FILE`), installed into the server image's system store at startup (or set `SSL_CERT_FILE` + `GIT_SSL_CAINFO` + `http.sslCAInfo`), applied to the `server` service only. Runners do not need it and the egress allowlist must not gain the GitLab host.
- Never ship an insecure-skip option. Fail with an actionable error that names the CA variable.
- Smoke test with an nginx container presenting a private-CA cert (docker/ smoke style), covering both `glab api` and `git fetch`.
- Also check the `glab` config `hosts.<host>.api_protocol`, `git_protocol` and the git credential helper: `glab auth login` can configure HTTP credential helpers that `git fetch` over HTTPS depends on.
**Detection:** smoke fails at first call; error text contains `x509` or `SSL certificate problem`.
**Phase:** Infra/auth phase (first phase after abstraction).

### 7. Abstraction regressions in the GitHub flow
**What goes wrong:** pulling 26 files off `github::` introduces GitHub behaviour changes: GitHub-shaped types leak into the neutral layer (`pull_number`, `node_id`, `owner/repo` pair, `side`), DB columns that store `owner/repo` get reinterpreted, URLs are built with `github.com`, UI says "PR" everywhere.
**Why it happens:** the easiest refactor is a trait mirroring GitHub's API, with GitLab forced into it, or the reverse; and "while I'm here" edits to the review pipeline.
**Prevention:**
- Neutral model first: `RepoRef { host, full_path }`, `ChangeRequest { number (iid), head_sha, base_ref, ... }`, `DiffAnchor { path, old_line, new_line }`. Keep the GitHub implementation byte-for-byte equivalent: move code, do not rewrite it.
- Do the refactor as its own phase with zero behaviour change, proven by the existing test suite and `docker/runner-smoke.py`, before any GitLab code lands.
- Provider chosen per deploy (PROJECT.md): use a single enum/config read once; do not scatter `if gitlab` across review code. Never touch `backend/src/review/` validation rules; they are about snapshots, not forges.
- Migrations append-only; do not rename existing `github_*` columns. Add new nullable columns or a provider discriminator with default `github`.
- Keep file-size limits: `publication.rs` (1444) and `chunks.rs` (2372) are near the cap; put GitLab code in new modules.
**Detection:** diff of GitHub test fixtures changes (should be none); grep `github::` count outside `backend/src/github/` goes to zero; `cargo test --workspace` unchanged results.
**Phase:** Phase 1 (abstraction), gate for everything else.

### 8. Token scopes (api vs read_api)
**What goes wrong:** read operations work in testing, posting discussions returns `403 Forbidden` or `401` in production, or the reverse: an over-privileged token is requested.
**Why it happens:** `read_api` allows reading MRs, versions, diffs; creating discussions/notes needs `api`. `read_repository` is needed for git over HTTPS (a PAT with `api` also grants it). Project/group access tokens have a role requirement (Reporter+ can comment; and the role is separate from the scope). Bot token expiry (max lifetime for PATs, 365 days default in newer versions) breaks a long-lived deploy silently.
**Prevention:** document required scope `api` (needed for write) in Settings and in the error text; run a "preflight" at sign-in: `GET /user`, `GET /projects/:id`, and a dry check of the access level (`permissions.project_access.access_level >= 20`). Surface token expiry (`GET /personal_access_tokens/self`, verify availability) as a warning. Token never to logs/browser; add a redaction test for `glpat-` patterns (CONCERNS: add redaction test per provider).
**Phase:** Auth phase.

## Moderate Pitfalls

### 9. iid vs id confusion
**What goes wrong:** MR endpoints take the project-scoped `iid` (shown as `!42`); the global `id` returns 404 or someone else's MR. For projects, `id` (numeric) vs path.
**Prevention:** neutral model carries `number = iid` only; name the field `iid` in GitLab structs; never persist the MR `id`. Add a test where `id != iid`. Link building uses `iid` (`/-/merge_requests/<iid>`).
**Phase:** Provider layer.

### 10. Namespaced paths: URL encoding and nested subgroups
**What goes wrong:** `group/sub/project` in `/projects/group/sub/project/merge_requests` is a 404. Encoding only once through a shell or `glab api` double-encodes (`%252F`). Splitting the path on `/` into `owner/repo` assumes two segments and breaks on subgroups. Case: path with dots, `.git` suffix, or renamed/moved projects (path changes, numeric id stable).
**Prevention:**
- Use the numeric project id for API calls after one lookup (`GET /projects/:url_encoded_path`), and store both id and `path_with_namespace`. Key DB rows on host + id where possible; display by path.
- Encode with a single percent-encoding of `/` -> `%2F`; when passing through `glab api`, pass the encoded path as-is and check that glab does not re-encode (test). `glab api` also substitutes `:id`/`:fullpath` placeholders for the current repo (verify), so avoid cwd-dependent forms; pass `-R`/hostname explicitly or the full path.
- Remote URL for git: `https://host/group/sub/project.git`; treat repository name as everything after the host.
- Handle project moves: on 404/301, re-resolve by id.
**Detection:** test project at `a/b/c/d`, containing a dot and a hyphen.
**Phase:** Provider layer, Repository access (listing groups and subgroups, include `include_subgroups=true` and `GET /groups/:id/projects`, including shared projects).

### 11. Pagination and rate limits on self-hosted
**What goes wrong:** default `per_page=20`, so only the first 20 MRs, projects, discussions or diff files are seen. Offset pagination is capped on large collections (keyset required for some endpoints; `X-Total` omitted for >10k results). Polling with N projects x M MRs triggers `429` if the admin enabled rate limits (authenticated API limits are off by default on self-managed but admins often enable them); `Retry-After` honoured by nobody. Idempotency check lists all discussions each time: on a busy MR (hundreds of notes) this is multiple pages.
**Prevention:** always set `per_page=100` and follow `Link: rel="next"` (or `X-Next-Page`) until empty; in `glab api` use `--paginate` and be aware it concatenates JSON arrays (verify behaviour with the pinned version). Poll `GET /merge_requests?state=opened&scope=all&updated_after=...` at group level instead of per-project when possible. Backoff on 429/5xx with `Retry-After`, bounded, with an actionable visible error (AGENTS: visible state). Cache the discussion listing per publication transaction.
**Phase:** Polling, Publication.

### 12. MR diff size limits / collapsed and too-large diffs
**What goes wrong:** the MR changes API returns `overflow: true` (older `/changes`) or entries with empty `diff` and `too_large`/`collapsed` flags; generated, large or binary files lack a patch; the instance's diff limits (max files, max lines, max patch bytes; admin-configurable) cut the list. A bot that parses the API diff will treat truncated files as "unchanged" and mis-anchor or quietly skip files; and `new_file`/`deleted_file`/`renamed_file` flags matter.
**Prevention:** do not use the REST diff as source of truth for the review. Compute the diff locally from the fetched refs (`git diff base...head`, as today's flow does with GitHub) so scope and anchors are identical on both providers. Use the API only for metadata (`diff_refs`, files list as cross-check). If a changed file cannot be anchored on GitLab because the instance collapses it, GitLab will reject an inline position there (verify): downgrade that finding to a general MR note with path and line in the body rather than failing the whole publication. Use `GET .../diffs` (paginated, newer) rather than the deprecated `/changes` when you do call it (verify deprecation status on target version).
**Detection:** smoke MR with a huge generated file and a binary file.
**Phase:** Review preparation; Publication (fallback path).

### 13. Renamed, moved, deleted, mode-only files
**What goes wrong:** `old_path` != `new_path` and sending only one gives 400; deleted files only have `old_line`; pure renames have no hunks so nothing to anchor; local `git diff` without rename detection shows delete + add while GitLab shows a rename, so the (old_path, new_path) pair disagrees with GitLab's diff.
**Prevention:** use the same rename detection as GitLab (`-M`, default similarity 50%; compare to the files in the version diff) and carry `old_path` and `new_path` in the neutral anchor. Add the path-completion rule from AGENTS.md ("cited path missing leading directories") careful: it must work on `new_path`.
**Phase:** Publication, Review preparation.

### 14. glab version differences and `glab api` quirks
**What goes wrong:** flags, output and defaults change between glab releases (`glab mr` subcommands, `--hostname`, config keys, `glab api --paginate` and `--input`, `-F` vs `-f` typed vs raw fields, how nested keys like `position[new_line]` are encoded). Passing integers via `-f` sends strings (usually accepted, but verify for `position[new_line]`); `-F` can read `@file` and treat `@` as file indicator, which corrupts note bodies starting with `@`. Newer glab versions may print update notices or prompts to stdout/stderr and the telemetry/update check calls out (blocked by egress or offline).
**Prevention:** pin glab with a checksum (already planned), set `GLAB_CHECK_UPDATE=false`, `NO_PROMPT=1`, `GLAB_SEND_TELEMETRY=false` (verify variable names against the pinned version), always pass `GITLAB_HOST` or `--hostname` explicitly. For POST bodies use `glab api --input -` with a JSON body piped on stdin (Content-Type application/json), never `-f` per field; this avoids shell quoting, `@` handling and nested key encoding issues, and note bodies containing newlines or `$`. Parse output as JSON only; treat non-zero exit plus JSON error body (`message`) as the error, and map 401/403/404/409/422/429 to actionable messages. Add a canary like the existing provider CLI canary (`provider-cli-canary.yml`) for glab output shapes.
**Phase:** Auth/CLI phase.

### 15. Note body length limit and Markdown handling
**What goes wrong:**
- Note/discussion body limit is 1,000,000 characters by default on recent versions but admin-tunable and older versions had 1M too; the real limit that bites is the rendering and the `413`/`422 Body is too long` on proxied instances (nginx `client_max_body_size`). The summary comment with suggestions and the full templates can be large.
- GitLab Flavored Markdown differs: `<!-- ... -->` HTML comments are preserved in the stored note body but hidden in render (good for markers), however they are visible if a user edits, and some GitLab versions/sanitizers strip them in *rendered* output while the API `body` keeps them (verify against the target instance). Markers must be read from `body`, not `body_html`.
- Quick actions: lines starting with `/` (e.g. `/approve`, `/merge`, `/assign`, `/label`) in a note body are executed as quick actions! A finding body quoting code that starts a line with `/approve` or `/merge`, or `/submit_review`, could trigger actions (or hide the line). Also `@user` mentions notify people, `#123` and `!45` auto-link to issues and MRs, and `:emoji:` converts.
- Code fences with triple backticks inside quoted snippets break the block; GitLab "suggestion" blocks (```suggestion) are interpreted specially on diff notes: a finding quoting code in a ```suggestion fence would render an apply button.
**Prevention:** neutral formatter with a GitLab post-processor: escape or neutralise lines beginning with `/` (zero-width/backslash or put all quotes in fenced code, but quick actions still apply to non-code lines only; verify that fenced blocks are immune), wrap user/code text in fenced blocks using a fence longer than any backtick run in the content, never emit a ```suggestion fence unless intended, escape `@` mentions in model text. Truncate with a visible "truncated" notice at a conservative limit (e.g. 60k chars; and split summary into multiple notes) while preserving the marker and full templates (AGENTS: posted findings keep the full templates). Test the marker round trip through create then list.
**Phase:** Publication.

### 16. Idempotency and bot identity
**What goes wrong:** publication retries create duplicate discussions (a POST may succeed server-side and time out client-side); there is no GitHub-style review object, so partial publication is observable by humans; finding the "status comment" to update requires knowing which notes are ours.
**Prevention:** record the discussion id in the DB right after each POST; on retry, list discussions and match by marker + fingerprint + author (`GET /user` id) before posting; use `PUT .../notes/:id` to update the status note. System notes (`system: true`) and resolved flag must be skipped. Respect `lock_pull_request` before state-dependent writes (AGENTS).
**Phase:** Publication.

### 17. Draft MRs and MR states
**What goes wrong:** `draft` (previously `work_in_progress`; title prefix `Draft:`, `WIP:` in old versions) are discovered or ignored inconsistently; the polling filter `wip=yes/no` was deprecated in favour of `draft` (verify on target version); closed, merged, locked MRs get reviewed; `merge_status` / `detailed_merge_status` is computed async and may be `checking`; the `merged` or `closed` state must stop publication.
**Prevention:** map state to the existing PR-selection settings (open, draft handling) in one function; use `state=opened` and read `draft` with fallback to the title prefix; re-check state just before publishing (closed/merged => skip, record why). Do not gate on mergeability.
**Phase:** Polling.

### 18. Authentication / host configuration quirks
**What goes wrong:** `glab auth login --hostname` writes `~/.config/glab-cli/config.yml`; in the container the home dir, the volume and the sign-in projection must be writable by the server but never mounted to runners (Sandbox invariants). Hostname given with scheme or trailing slash; instance under a relative URL root (`https://host/gitlab`), which breaks hostname-only configuration (verify glab support for `api_host`/relative root); SSH vs HTTPS `git_protocol` mismatch; non-default ports.
**Prevention:** validate `OCR_GITLAB_HOST` (host[:port], no scheme; decide how to handle a path prefix, or document unsupported); store glab config in the existing credential volume pattern used for `gh`; a preflight that runs `glab api version` equivalent and `git ls-remote` on one project.
**Phase:** Auth phase.

## Minor Pitfalls

### 19. UI vocabulary and links
**What goes wrong:** "PR" strings and `#123` shown for GitLab; link `https://host/<path>/-/merge_requests/<iid>` vs `/pull/<n>`; `!iid` notation.
**Prevention:** one provider-keyed terms and URL helper in the frontend; Vitest for both providers. Keep every file under 800 lines (`ReviewsPage.tsx` is at 782, `reviewFlow.ts` at 779: extract before adding).
**Phase:** UI phase.

### 20. Indexer and Repository access listing
**What goes wrong:** project listing returns only member projects of the first page; archived projects, projects with disabled MRs or empty repos appear; `min_access_level` unspecified; projects the token can read but not comment on.
**Prevention:** list with `membership=true`/`min_access_level=20` (verify param), skip `archived`, `empty_repo`, and `merge_requests_access_level=disabled`; paginate; show group/subgroup tree from full path.
**Phase:** Repository access phase.

### 21. Time and clock for polling
**What goes wrong:** `updated_after` compared against the bot's clock on a skewed instance misses MRs; a push does not always bump MR `updated_at` immediately.
**Prevention:** detect new head by comparing `sha`/`diff_refs.head_sha` per open MR, not by timestamps alone; overlap the window.
**Phase:** Polling.

## Phase-Specific Warnings

| Phase topic | Likely pitfall | Mitigation |
|-------------|---------------|------------|
| Provider abstraction | GitHub regressions, GitHub-shaped neutral types (7) | Zero-behaviour-change refactor first; existing tests and smokes as gate; new modules to respect file caps |
| Auth, glab, CA | TLS trust in three stores, glab flags, scopes, host format (6, 8, 14, 18) | One CA bundle env for server only; pin glab; preflight with actionable errors; no insecure flag |
| Polling | Pagination, 429, draft/state, head detection (11, 17, 21) | `per_page=100` + Link following; compare SHAs; backoff |
| Review preparation | MR refs, forks, diff source, renames (5, 12, 13) | Fetch `refs/merge-requests/<iid>/head`, verify OID vs API, local `git diff`, fail on mismatch |
| Publication | SHAs, line mapping, outdated positions, markers, quick actions, length (1-4, 15, 16) | Versions-derived SHAs, table-driven anchor map, marker-based idempotency, body sanitiser, fallback to general note |
| Repository access / indexing | Nested subgroups, path encoding, iid vs id (9, 10, 20) | Numeric id after one lookup; test with 3-level subgroup |
| UI | Terminology, 800-line cap (19) | Provider terms helper; extract components |

## Verification Needed Against Live Instance (smoke checklist)

1. Create discussion on added, removed, context line; on a renamed file; on a fork MR.
2. Push after review, force-push, then publish: confirm the abort path.
3. HTML-comment marker survives `GET discussions`.
4. A body line `/merge` in a finding is inert.
5. Private-CA instance: `glab api`, `git fetch`, no skip-verify.
6. Pagination with more than 100 MRs and more than 100 notes.
7. Token with `read_api` only: failure message names `api` scope.

## Sources

- Project: `.planning/PROJECT.md`, `.planning/codebase/CONCERNS.md`, `AGENTS.md` (HIGH for codebase facts).
- GitLab REST docs (Discussions API, Merge Requests API and versions, Notes API, Pagination, Rate limits, Personal access token scopes, Quick actions), glab docs: from prior knowledge, NOT re-fetched in this session (MEDIUM/LOW). Verify against `https://docs.gitlab.com/ee/api/` for the target instance version before implementation.
