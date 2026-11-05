# Phase 4: Escrita GitLab: publicação dos resultados - Research

**Researched:** 2026-11-05
**Domain:** GitLab write path (discussions, notes) behind the existing forge seam, Rust, `glab api`
**Confidence:** MEDIUM (codebase facts HIGH; GitLab/glab wire behaviour MEDIUM, no real instance available)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
**Inline (PUB-01/02)**
- Antes de postar: `GET projects/:id/merge_requests/:iid/versions`, usar a versão mais recente (`base_commit_sha`, `start_commit_sha`, `head_commit_sha`) para `position`; nunca SHAs do git local.
- Linha adicionada: só `new_line`; removida: só `old_line`; contexto: ambos. `old_path` e `new_path` sempre. `position_type=text`.
- `POST .../discussions` com corpo JSON via `glab api --input <arquivo>`, nunca `-f/-F`. Conferir que a nota criada tem `type == "DiffNote"`; se não, ou se vier erro 400 de posição, postar como nota geral com o mesmo marcador `<!-- ocr-... -->` e o caminho/linha no texto.
- Uma linha por finding (multi-linha `line_range` fica para v2).

**Status e resumo (PUB-03/04)**
- Nota de status: procurar nota com o marcador de status (`GET .../notes`, ler `body`, não `body_html`); se existe, `PUT` no lugar; senão cria. Mesmo texto do GitHub.
- Resumo: nota geral com o marcador da revisão, postada por último; o id dela é o "review id" para idempotência. Seção de sugestões quando Post suggestions está ligado, igual ao GitHub.
- A impl GitLab devolve ids/estados no formato que `publication.rs` já espera (ids BIGINT), para não ramificar lá; código novo em módulo novo.

**Idempotência (PUB-05)**
- Antes de postar, listar discussões/notas existentes do MR e pular findings cujo marcador já existe. Retry após falha parcial reposta só o que falta.

**Head e segurança do texto (PUB-06/07)**
- Re-ler o MR (`sha`) logo antes de postar; se diferente do head revisado, abortar com erro claro (mesma semântica do GitHub).
- Pós-processador do corpo para GitLab: linhas que começam com `/` não podem virar quick action (prefixar com zero-width space); `@user`, `#123`, `!123`, `~label`, `%milestone` neutralizados com zero-width space após o sigilo; blocos de código continuam legíveis; fence `suggestion` vira `diff`. Teste unitário por caso.
- Truncar corpo no limite do GitLab (1.000.000 caracteres) com aviso.

### Claude's Discretion
Nomes, layout de `backend/src/gitlab/publish*.rs`, forma exata do escape. Velocidade acima de polimento.

### Deferred Ideas (OUT OF SCOPE)
- Multi-linha (`line_range`), sugestões aplicáveis inline, auto-resolve → v2.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PUB-01 | Inline discussion on correct line (added/removed/context) | `diffs.rs` line map from `GET .../merge_requests/:iid/diffs`; position rules below |
| PUB-02 | Rejected position -> general note with same marker | `ApiWriteError::Rejected` + DiffNote check + fallback body |
| PUB-03 | Status note created once, edited in place | `StatusCommentApi` GitLab impl + new `update` hook; see "Status note design" |
| PUB-04 | Summary posted as note (+ suggestions) | `post_review` posts summary last; body already built by `publication.rs` (suggestions included) |
| PUB-05 | No duplicates on retry | fingerprint list-and-skip inside `post_review` + compose-time `dedupe_against` |
| PUB-06 | Abort on push after review | existing `validate_live_review_scope` + new MR `sha`/`versions` check in `post_review` |
| PUB-07 | No quick actions / mentions / autolinks | `gitlab/sanitize.rs` applied to every outgoing body |
</phase_requirements>

## Summary

`publication.rs` is almost provider-neutral already: it builds a GitHub-shaped payload (`{commit_id, event, body, comments:[{path,line,side,body}]}`, body ends with the `<!-- ocr-review-post:v1 ... -->` marker, each comment body starts with `<!-- ocr:fingerprint=fnv64:... -->`) and hands it to `forge::prepare_review` -> `forge::post_review` -> `forge::find_review_id_by_marker`. Those names currently fall through the `pub(crate) use crate::github::*` glob in `backend/src/forge/mod.rs`, so on GitLab they hit `gh` and fail closed (`require_github`). The cheapest correct design is: **keep the GitHub payload shape, add forge-dispatch wrappers (same trick Phase 3 used) that route to new `backend/src/gitlab/publish*.rs`**, and have the GitLab `post_review` translate the payload into discussions + a final summary note. `publication.rs` then changes only by deleting the Phase-3 early return (lines 555-561).

The GitLab `post_review` does everything GitLab-specific: head/versions check, list existing notes/discussions (fail closed), per-finding POST discussion with `position` (fallback to general note), summary note last, returns the summary note id as the review id. Every outgoing body goes through `sanitize::finalize_body` (quick actions, sigils, suggestion fence, 1,000,000-char cap with markers preserved).

**Primary recommendation:** Add forge wrappers for `post_review`, `find_review_id_by_marker`, `diff_anchors`, `dedupe_review_comments`, `snapshot_review_requests`, `restore_review_requests`, `upsert_issue_comment_by_marker` and a local `GitHubStatusComments` shadow type; implement the GitLab side in 5 new files under `backend/src/gitlab/`; use `glab api -X POST -H "Content-Type: application/json" --input <file>` (the header is REQUIRED, see Pitfall 1).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Posting discussions/notes | API/Backend (server process, `glab`) | — | Runners never get hosting credentials |
| Position computation (old/new line) | API/Backend | — | From GitLab `/diffs`, not local git |
| Body sanitising | API/Backend | — | Last step before any write, covers all bodies |
| Head-staleness check | API/Backend | — | Live MR read right before write |
| Idempotency state | Database (review id, comments_json) + GitLab markers | — | Marker in GitLab is the source of truth for recovery |

## Standard Stack

No new crates and no new packages. `serde_json`, `serde`, existing `glab` 1.128.0 (pinned with checksum in `docker/Dockerfile`). Do NOT add `regex` (not in `backend/Cargo.toml`); write the sanitizer as a hand-rolled line/char scanner. Package Legitimacy Audit: not applicable (no installs).

## Existing code map (what the GitLab impl must satisfy)

### Dispatch layer
`backend/src/forge/mod.rs`: `current()`, `host()`, `Forge`, wrapper pattern `match current() { GitHub => crate::github::X, GitLab => crate::gitlab::X }` shadowing the glob. `require_github(what)` returns `"{what} is not supported yet on GitLab."`; `github/transport.rs` lines 8 and 118 call it so any non-dispatched `gh` call fails closed.

### Call sites that need a GitLab implementation (exact signatures)

| Wrapper to add in `forge/mod.rs` | GitHub signature (must match) | Callers | GitLab behaviour |
|---|---|---|---|
| `prepare_review(&str) -> Result<PreparedReview,String>` | no wrapper needed | publication.rs:1030, review_http.rs:156 | Reuse as is (temp file, 0600, Drop cleanup). Add `pub(crate) fn path(&self) -> &Path` to `PreparedReview` (field is private, `github/comments.rs`). |
| `post_review(&str, i64, PreparedReview) -> Result<Option<i64>,String>` | comments.rs:69 | publication.rs:1034, review_http.rs:162 | `gitlab::post_review`: parse payload, publish, return `Some(summary_note_id)` |
| `find_review_id_by_marker(&str, i64, &str) -> Result<Option<i64>,String>` | comments.rs:80 | publication.rs:1035, review_http.rs:163, runtime.rs:292 and :431 | List notes (`GET .../notes?per_page=100&sort=asc`), return id of first note whose `body` contains the marker. Errors propagate (ambiguity keeps lock, existing semantics). |
| `diff_anchors(&str, i64) -> HashMap<String,(HashSet<i64>,HashSet<i64>)>` | pr.rs:8 | publication.rs:1271 | Parse `GET .../diffs` (right = new lines of added+context, left = old lines of removed+context). Empty map on failure (existing "skip checks" contract). |
| `dedupe_review_comments(&str, i64, Value) -> DedupeResult` | comments.rs:223 | publication.rs:1333 | `crate::github::dedupe_against(&gitlab_existing, comments)`; needs re-export of `dedupe_against` + a constructor for `ExistingOcrComment` (fields are private; add `pub(crate) fn from_parts(author,url,path,line,body)` using `extract_ocr_fingerprint`/`point_title`). |
| `snapshot_review_requests(&str,i64) -> Result<ReviewRequestSnapshot,String>` | review_requests.rs:155 | publication.rs:1028, review_http.rs:148 | Return a default snapshot (`login:""`, empty sets). Row in `review_request_restorations` is harmless. |
| `restore_review_requests(&str,i64,i64,&ReviewRequestSnapshot) -> Result<bool,String>` | review_requests.rs:229 | runtime.rs:439 | `Ok(true)` (nothing to restore). Worker already runs on GitLab (`start_review_request_restoration`, server/mod.rs:296). |
| `upsert_issue_comment_by_marker(&str,i64,&str,&str) -> Result<IssueCommentUpsert,String>` | comments.rs:101 | summary.rs:70 (PR summary comment) | find note by marker -> `PUT` else `POST`; return `IssueCommentUpsert{id,created}` (add `IssueCommentUpsert` to the `pub use comments::{..}` list in github/mod.rs). |
| `GitHubStatusComments` (unit struct) | status_comments.rs:17 | publication.rs:395, queue_acknowledgements.rs:99, runtime.rs:181, status_comments.rs:189 | Define a **local** `pub(crate) struct GitHubStatusComments;` in `forge/mod.rs` (explicit item shadows the glob re-export) implementing `StatusCommentApi` by dispatch. Zero call-site edits. |

Everything else on the path (`live_pr_state`, `current_login`, `commit_url`, `review_url`) is already forge-aware.

### Gates to lift (the only "not supported / skip on GitLab" in the write path)
1. `publication.rs:555-561` in `handle_completed_review`: early return + lane "ready" "Publishing to GitLab arrives in Phase 4". Delete the block (file shrinks).
2. `queue_acknowledgements.rs:163-165` in `spawn_queue_acknowledgements`: GitLab early return. Remove once the GitLab `StatusCommentApi` exists.
3. `comment_activation.rs:8` `queued_for_review_text` builds `https://{gh_host}/{full_name}/commit/{head}` (GitHub URL shape; GitLab needs `/-/commit/`). Use `forge::current().commit_url(..)` only when `current()==GitLab` (GitHub path uses `gh_host()` for GHE, and `Forge::GitHub.commit_url` hardcodes github.com, so do not swap it for GitHub).
4. `decide_review_http` (review_http.rs:71-76) and `auto_post_event`/mode handling: GitLab must never approve or request changes. Reject `approve_post`/`request_changes` on GitLab with a clear error (error text there says "GitHub does not allow..." for the own-PR case), and treat `auto_post_mode=request_changes` as `comment` on GitLab. `gitlab::post_review` must ignore `event` entirely and never call approve endpoints (also keeps `db.set_ocr_review` unused).
5. `retire_clean_comments_after_findings` / `replace_clean_comment` already flow through the shadow type.

### Idempotency/DB facts
- Review id = `Option<i64>` end to end (`github_review_id` BIGINT column, `reserve_effective_review_result(.., review_id, review_url, ..)`). GitLab note ids are integers: fits. `review_url` for GitLab ignores the id (`gitlab_review_url`); optional improvement `#note_{id}` (existing test `gitlab_urls_and_refspec` asserts the plain URL, update it if changed).
- Lock/recovery: `db.try_begin_review_post` -> `publish_review_with_recovery(prepare, post, reconcile, reset)` -> on POST error calls `find_review_id_by_marker`; `None` => `reset_review_posting` and Err; `Err` => stays locked for startup reconciliation (`runtime.rs:279 reconcile_interrupted_review_posts`). The summary note (with `<!-- ocr-review-post:v1 repo pr run head installation -->`) is posted LAST, so its presence proves a complete publication; its absence after partial inline posting resets and a retry resumes via fingerprint skip.
- Markers in play: `<!-- ocr:fingerprint=fnv64:XXXX -->` (first line of each inline body, `with_fingerprint_marker`), `<!-- ocr-review-post:v1 ... -->` (summary), `<!-- ocr-pr-summary:v1 ... -->` (PR summary comment), `<!-- ocr-queue:v1 generation=N -->` + activation marker (queued ack), result marker key + `ocr-result-data` (clean result). All start with `<!--` at line start: the sanitizer must leave those lines untouched.
- Head check for GitHub: `validate_live_review_scope` (publication.rs:33-76) before/after `prepare`, `confirm_posted_review_live_head` after posting (marks superseded). It uses `forge::live_pr_state` which already works for GitLab (Phase 3). Reuse; do not duplicate.
- Comment JSON consumed by the poster: `path`, `line`, `side` ("RIGHT"|"LEFT"), `body` (+ ignored `start_line`, `start_side`).

### Existing `gitlab/` module
- `api.rs`: `api_at(bin, host, dir, endpoint, paginate, what, backoff_ms, cancel)` is GET only (`glab api [--paginate] <endpoint>`), retries 429/5xx up to 3x, returns FIXED error strings (generic message, no stderr), `parse_pages::<T>` merges concatenated arrays, `enc()` percent-encodes. `run_glab_cancellable(bin, host, dir, args, stdin, timeout, cancel)` in `auth.rs`; env (`GITLAB_HOST`, `GLAB_CONFIG_DIR`, tokens removed) is set there, so the token is never in argv. `glab_read()` lock guards each attempt.
- **No POST/PUT helper exists.** Add `api_write_at(bin, host, dir, method, endpoint, body_file: &Path, what, backoff_ms, cancel) -> Result<Vec<u8>, WriteError>` in a new `gitlab/write.rs` (api.rs is 101 lines, fine either way) building args `["api","-X",method,"-H","Content-Type: application/json","--input",path,endpoint]`.
  - `enum WriteError { Rejected(String /*fixed text*/), Other(String) }`. `Rejected` when the combined stdout+stderr contains token `400`, `409` or `422` (use the same alnum-token split as `retryable`). Never include server text in the returned string (existing rule).
  - **Do not auto-retry POST on 5xx or timeout** (server may have created it; recovery is by marker listing). Retry only on `429`. PUT may retry like GET.
  - 401/403 -> `TOKEN_MESSAGE` via `classify` (as in `api_at`). A 403 on write means the token lacks `api` scope or role below Reporter: give an actionable fixed message.
- Fake-glab test pattern: `read_tests.rs` `mod fake` (`#[cfg(unix)]`): writes `/tmp/.../glab` shell script that appends `"$*"` to `args.log` and `case "$*"` on exact arg strings; `calls(dir)`. For writes the body is in the `--input` file which is deleted after the call, so the fake script must copy it: `prev=; for a in "$@"; do [ "$prev" = --input ] && cat "$a" >> "$(dirname "$0")/bodies.log"; prev=$a; done`. Also assert `args.log` never contains a body or `glpat-`.

## GitLab API facts needed

- Versions: `GET projects/:id/merge_requests/:iid/versions` -> array, latest FIRST; fields `head_commit_sha`, `base_commit_sha`, `start_commit_sha`. [CITED: docs.gitlab.com/api/discussions]. Version creation after a push is asynchronous: if `versions[0].head_commit_sha != payload.commit_id` while MR `sha == commit_id`, treat as transient failure (Err, retried later), never post with mismatched SHAs.
- Create thread: `POST projects/:id/merge_requests/:iid/discussions` JSON `{"body": "...", "position": {"position_type":"text","base_sha","start_sha","head_sha","old_path","new_path","old_line"?,"new_line"?}}`. Added line: `new_line` only; removed: `old_line` only; unchanged: both; `old_path` and `new_path` both required. [CITED: docs.gitlab.com/api/discussions]. Success 201 with `{id (string), individual_note:false, notes:[{id, type:"DiffNote", position:{..}}]}`. Docs do not specify the invalid-position error code; expect 400 (PITFALLS.md) and treat 400/422 as `Rejected`.
- Context line needs BOTH numbers, so `diff_anchors`' two sets are not enough: the poster must build `Vec<DiffLine{old: Option<i64>, new: Option<i64>}>` per file. Reuse `crate::github::parse_hunk_starts` (currently `pub(crate)` in private `mod pr`; re-export it) to walk the unified diff of each file.
- Diffs: `GET projects/:id/merge_requests/:iid/diffs?per_page=100` (paginated, GitLab >= 15.7; `/changes` is deprecated). Per file: `old_path`, `new_path`, `new_file`, `renamed_file`, `deleted_file`, `diff` (hunks starting at `@@`, no `---/+++` headers), possibly `collapsed`/`too_large` with empty `diff` => no anchors => finding goes to the general-note fallback. [ASSUMED: field names from training; verify on first real MR (HUMAN-UAT)].
- Notes list: `GET projects/:id/merge_requests/:iid/notes?per_page=100&sort=asc` -> `[{id, body, author:{id,username}, system, ...}]`; skip `system==true`. Read `body`, never `body_html`. Create `POST .../notes` `{"body"}`; edit `PUT .../notes/:note_id` `{"body"}`; delete `DELETE .../notes/:note_id` (own notes). Discussions list `GET .../discussions?per_page=100` -> `[{id, individual_note, notes:[{id, body, type, position?, system, author}]}]`; needed to see inline positions for dedupe. Current user id: `GET user` -> `id`.
- Body limit 1,000,000 characters. [CITED: .planning/research/PITFALLS.md, from training; MEDIUM]. A reverse proxy (`client_max_body_size`) can lower it in practice.
- Quick actions: execute when a line starts with `/command` in notes created through the API too. Whether they are ignored inside code fences/blockquotes is NOT confirmed by official docs fetched here [ASSUMED]; therefore neutralise leading `/<letter>` everywhere (also inside fences) at the cost of a zero-width space in a code line.
- **glab `--input` does not set Content-Type**: without `-H "Content-Type: application/json"` GitLab answers HTTP 415. [CITED: gitlab.com/gitlab-org/cli/-/work_items/8316 and /8517 via search; MEDIUM]. Also `-f` silently drops bracket-nested keys (`position[new_line]`), which confirms the locked `--input` decision. Verify against pinned 1.128.0 in HUMAN-UAT.

## Architecture Patterns

### Flow (auto and manual publication)
```
handle_completed_review / decide_review_http
  -> compose_review_post
       forge::diff_anchors      -> gitlab /diffs  (anchor sets)      [split relocated vs inline]
       forge::dedupe_review_comments -> gitlab discussions list -> dedupe_against
  -> build_review_payload (GitHub-shaped JSON, markers inside)
  -> publish_review_with_recovery
       prepare: forge::snapshot_review_requests (no-op) ; forge::prepare_review (temp file)
       post:    forge::post_review -> gitlab::post_review
                  1. read payload; live MR: state==opened && sha==commit_id else Err (stale)
                  2. versions[0].head_commit_sha==commit_id else Err
                  3. list discussions+notes (fail closed) -> set of fingerprints, summary-marker present?
                  4. diffs -> line map
                  5. for each comment (skip fingerprint already present):
                       sanitize body -> POST discussions {position} 
                       Rejected / type!=DiffNote -> general note (or PUT created note) with "path:line" header
                  6. re-check MR sha; summary: POST note (sanitized, marker last) unless marker already present
                  7. return Some(summary_note_id)
       reconcile: forge::find_review_id_by_marker -> notes list
  -> confirm_posted_review_live_head (existing)
```

### Recommended structure (all new, each well under 2,500 lines)
```
backend/src/gitlab/
├── write.rs        # api_write_at, WriteError, temp-body helper using PreparedReview
├── diffs.rs        # parse /diffs -> per-file line map; diff_anchors; position builder
├── publish.rs      # post_review, find_review_id_by_marker, upsert_issue_comment_by_marker, dedupe_review_comments, existing-comment listing
├── notes.rs        # GitLabStatusComments (StatusCommentApi), list/post/put/delete note helpers
├── sanitize.rs     # finalize_body + tests inline or sanitize_tests.rs
└── publish_tests.rs# fake-glab tests (#[cfg(unix)]) + pure tests
```
Register in `gitlab/mod.rs` the same way `read_tests` is (`#[cfg(test)] #[path = "publish_tests.rs"] mod publish_tests;`).

### Position builder (pure, table-driven)
`position_for(entry: &FileDiff, side: &str, line: i64) -> Option<Pos{old_line:Option,new_line:Option}>`: RIGHT+added -> new only; LEFT+removed -> old only; line present as context (either side) -> both numbers; not in diff -> `None` => general-note fallback directly (no POST attempt). `old_path`/`new_path` come from the diff entry (renames: path from payload equals `new_path`; deleted file: both equal).

### Status note design (PUB-03)
The queue/clean-result comments go through `StatusCommentApi { publisher_id, list, post, delete }` + `replace_owned_comment` (adopt same generation, else post, then delete older). GitHub impl cannot edit; GitLab must edit in place.
- Add a defaulted trait method `fn edit(&mut self, _repo:&str, _number:i64, _id:i64, _body:&str) -> Result<bool,String> { Ok(false) }` (GitHub unchanged) and `fn delete` gains a `number` param (GitLab delete URL needs the MR iid; GitHub impl ignores it; update the fake in `status_comments_test_support.rs` and the 3 call sites in `status_comments.rs`/`queue_acknowledgements.rs`).
- In `replace_owned_comment`: if no comment of this generation exists and an older owned comment exists, call `api.edit(.., oldest_owned_id, body)`; when it returns `true`, keep that id and skip post/delete. GitHub's default `Ok(false)` leaves its behaviour byte-identical (guard with the existing status_comments tests).
- `GitLabStatusComments`: `publisher_id` = `GET user` id; `list` = notes list mapped to `StatusComment{id, author_id: author.id, body}` skipping system notes; `post` = POST note; `edit` = PUT; `delete` = DELETE (404 = ok, like GitHub).
- Caveat/Open question: the queued-ack family and the clean-result family use different ownership markers, so GitLab can show two notes over time (ack edited per generation, then deleted at retire; result note posted once, edited on re-review). If the user wants literally one note, give GitLab one shared status marker in `queued_for_review_text`/`review_result_body` callers; not recommended for v1.

### Sanitizer spec (`finalize_body(body: &str) -> String`)
Order: (1) split into lines, track fenced code state (``` or ~~~ fences; fence closes on a line of >= the opening length of same char); (2) per line: if trimmed line starts with `<!--` leave untouched; else if first non-space char is `/` followed by an ASCII letter, insert U+200B right after the leading whitespace (everywhere, including fences); (3) outside fences and outside inline-code spans (odd backtick segments), after each sigil in `@ # ! ~ % $ &` that is preceded by start/non-alphanumeric and followed by `[A-Za-z0-9_]` (for `#`/`!` followed by digit or letter), insert U+200B right after the sigil; (4) fence info string starting with `suggestion` (lowercase, any suffix like `suggestion:-0+0`) rewritten to `diff`; (5) cap at 1,000,000 chars (count `chars()`, not bytes): when over, cut to fit and append `\n\n_Truncated by OpenCodeReview: the original text exceeded GitLab's note size limit._` and re-append the trailing marker lines (any final lines starting `<!--`) so idempotency still works. Apply to: inline body, fallback body, summary note, status notes, PR-summary comment, `PUT` bodies. Tests (one per case): leading `/merge`, indented `  /approve`, `//` comment not touched, `@alice`, `a@b.com` (email must not be broken needlessly: preceded by alnum -> untouched), `#12`, `!5`, `~bug`, `%1.0`, inline code span untouched, fenced block untouched except leading `/`, marker lines untouched, ```` ```suggestion ```` -> ```` ```diff ````, multibyte truncation at char boundary, marker preserved after truncation.
When reading back existing bodies for dedupe/ownership, strip U+200B first (`body.replace('\u{200B}', "")`) so `point_title`/status parsing still match.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---|---|---|
| Temp request file (0600, unique, Drop cleanup) | new temp code | `crate::github::prepare_review(json)` -> `PreparedReview` (+ `path()` accessor) |
| Dedupe algorithm | GitLab-specific matching | `crate::github::dedupe_against` + `ExistingOcrComment` |
| Hunk parsing | new parser | `parse_hunk_starts` (re-export) |
| Pagination merge | own parser | `api::parse_pages::<T>` |
| URL encoding | own | `api::enc` |
| Retry/ownership of comments | new | `replace_owned_comment` / `owned_comment_generations` |
| HTTP/JSON | reqwest, glab flags `-f` | `glab api --input` only |

## Common Pitfalls

1. **Missing `Content-Type`** with `--input` -> 415 on every write. Always pass `-H "Content-Type: application/json"`; assert it in the fake-glab test args.
2. **Shell-visible bodies:** body must only be in the temp file, never in argv (fake test: `args.log` contains no finding text).
3. **POST retried after 5xx/timeout** duplicates the discussion. No auto-retry for POST except 429; recovery by listing (fingerprint/marker).
4. **Context lines posted with one number** => GitLab 400 or wrong anchor. Context needs old+new, hence the per-file line map (not `diff_anchors` sets).
5. **Created note is not a `DiffNote`** (position silently dropped): PUT the created note to add the `path:line` header instead of posting a second note (avoids a duplicate with the same marker).
6. **Marker mangled by sanitizer:** skip any line starting `<!--`; never insert ZWSP inside the marker. Strip ZWSP when reading bodies back.
7. **Versions lag after push:** versions[0].head != reviewed head even though MR `sha` matches; return Err (transient), do not fall back to local SHAs.
8. **Outdated discussions:** an existing marker whose position head differs is still treated as "already posted" (parity with GitHub, which counts outdated comments via `original_line`). PITFALLS.md suggested treating them as stale; flagged as Open Question 1.
9. **File-size/regression:** `publication.rs` is 1,454 lines: net change should be a deletion. Do not branch on `Forge` there.
10. **Rate/limit:** large MR = many POSTs; `429` handling with `Retry-After`-agnostic bounded backoff (reuse `backoff_ms << attempt`, 3 attempts). Listing is done once per `post_review`, not per finding.
11. **`approve`/request-changes on GitLab:** ignore `event` in `post_review`; guard decisions at `decide_review_http`; otherwise `db.set_ocr_review` records a review GitLab never got.
12. **Token leakage:** error strings from the write helper are fixed text; no stderr/stdout; reuse `redact` if any detail is ever logged.

## Code Examples

```rust
// write.rs sketch: args for a JSON write (no body or token in argv)
let args = ["api", "-X", method, "-H", "Content-Type: application/json", "--input", &path, endpoint];
// run via run_glab_cancellable(bin, host, dir, &args, None, Duration::from_secs(120), cancel)

// position JSON (context line)
serde_json::json!({
  "body": sanitized,
  "position": {"position_type":"text","base_sha":v.base_commit_sha,"start_sha":v.start_commit_sha,
               "head_sha":v.head_commit_sha,"old_path":old,"new_path":new,"old_line":o,"new_line":n}
})
```
Dispatch wrapper (same shape as existing ones):
```rust
pub(crate) fn post_review(full_name: &str, number: i64, prepared: PreparedReview) -> Result<Option<i64>, String> {
    match current() { Forge::GitHub => crate::github::post_review(full_name, number, prepared),
                      Forge::GitLab => crate::gitlab::post_review(full_name, number, prepared) }
}
```

## State of the Art / Assumptions

| # | Claim | Risk if wrong |
|---|-------|---------------|
| A1 | `/merge_requests/:iid/diffs` returns `old_path/new_path/diff/collapsed/too_large` per file (GitLab >= 15.7) | Anchor map empty -> everything falls to general notes; fixture must be revalidated on UAT |
| A2 | Invalid position yields HTTP 400 (maybe 422) | Fallback not triggered on a different code; mitigation: treat 400/409/422 as Rejected and any non-DiffNote 201 as fallback |
| A3 | Quick actions are not processed in fenced code | Sanitizer neutralises leading `/` everywhere anyway |
| A4 | `glab api -X POST -H ... --input file` works on glab 1.128.0 and prints JSON on success; on HTTP errors exits non-zero with status text in stderr or stdout | Rejected detection must scan both streams; UAT |
| A5 | Note body limit 1,000,000 chars | Lower proxy limits -> 413; surface as generic write error |
| A6 | GitLab sigils to neutralise include `$` and `&` beyond the five locked ones | Cosmetic only |

## Open Questions

1. Outdated (old-head) bot discussions: parity with GitHub (skip, recommended) vs re-post. Decide at plan time; default parity.
2. One shared status note vs two families (ack + result). Default: two families, ack edited in place.
3. Should `request_changes` auto-post mode be hidden in the UI on GitLab? Backend coerces to comment; frontend copy is optional.

## Environment Availability

| Dependency | Required By | Available | Notes |
|---|---|---|---|
| glab 1.128.0 | runtime | in server image (pinned) | No real GitLab instance; fake `glab` shell script for tests |
| WSL + Docker | Rust tests (`~/rusttest.sh`) | yes (per task context) | starts throwaway Postgres |

## Validation Architecture

### Test Framework
| Property | Value |
|---|---|
| Framework | `cargo test` (Rust 1.98.1), inline `#[test]`, fake `glab` shell scripts (`#[cfg(unix)]`) |
| Config file | `backend/Cargo.toml`; DB tests need `OCR_TEST_DATABASE_URL` (handled by `~/rusttest.sh`) |
| Quick run command | `wsl -e bash -lc '~/rusttest.sh "--lib gitlab"'` |
| Related suites | `... "--lib forge"`, `... "--lib server::"` (status_comments, queue_acknowledgements, publication), `... "--lib github"` (no regression) |
| Full suite | `wsl -e bash -lc '~/rusttest.sh'` plus `cargo fmt --check`, `cargo clippy -- -D warnings`, `pnpm check:file-size`, changelog fragment in `changelog.d/` |

### Phase Requirements -> Test Map
| Req | Behavior | Type | Command / location | Exists |
|---|---|---|---|---|
| PUB-01 | position for added/removed/context/renamed/deleted; DiffNote asserted; JSON body has right keys | unit + fake glab | `--lib gitlab` : `diffs` + `publish_tests` | Wave 0 |
| PUB-02 | 400 on POST and `type != DiffNote` both end as general note with marker and `path:line` | fake glab | `--lib gitlab` | Wave 0 |
| PUB-03 | status note created once, second call PUTs same id; GitHub `replace_owned_comment` behaviour unchanged | unit (`status_comments_tests.rs` fake) + fake glab | `--lib server::status` and `--lib gitlab` | Wave 0 (extend fake with `edit`) |
| PUB-04 | summary note posted last with marker; suggestions section present when body has it; returns note id | fake glab | `--lib gitlab` | Wave 0 |
| PUB-05 | second run with existing fingerprints posts 0 discussions; partial failure then retry posts only the rest; marker find returns id | fake glab with stateful script | `--lib gitlab` | Wave 0 |
| PUB-06 | MR `sha` != commit_id -> Err before any POST; versions head mismatch -> Err; sha changes before summary -> Err | fake glab | `--lib gitlab` | Wave 0 |
| PUB-07 | sanitizer cases (list in spec); no body/token in argv | unit | `--lib gitlab sanitize` | Wave 0 |
| Regression | GitHub flow untouched; forge dispatch tests | existing | `--lib forge`, `--lib github`, `--lib server::` | yes |

### Sampling Rate
- Per task commit: `--lib gitlab` (+ `--lib forge` when forge/mod.rs changes).
- Per wave merge: `--lib server::` and `--lib github`.
- Phase gate: full `~/rusttest.sh`, clippy, fmt, file-size, then `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `backend/src/gitlab/publish_tests.rs` (fake glab with body capture to `bodies.log`)
- [ ] sanitizer tests (inline in `sanitize.rs` or `sanitize_tests.rs`)
- [ ] extend `server/status_comments_test_support.rs` fake with `edit` and `delete(number)`
- [ ] HUMAN-UAT items (no real instance): real `diffs` shape, real invalid-position status code, `--input` Content-Type on glab 1.128.0, quick-action behaviour in fences, internal CA path for writes

## Security Domain

| ASVS | Applies | Control |
|---|---|---|
| V5 Input validation / output encoding | yes | `sanitize::finalize_body` on all model-derived text; path/line validated against diff map |
| V6 Cryptography | no | none added |
| V7 Error handling/logging | yes | fixed error strings, no stderr/body, `redact` |
| V4 Access control | yes | never approve/merge; only notes/discussions; runner never sees token (unchanged) |

Threat patterns: model text injecting quick actions (`/merge`, `/approve`, `/assign`) -> leading-`/` neutralisation; mention spam (`@group`) -> sigil ZWSP; token in argv/logs -> env-only `glab`, body via temp file 0600; stale-position posting -> head/versions check; duplicate spam -> fingerprint skip.

## Project Constraints (from CLAUDE.md)
- Docker Compose only; `glab` pinned (1.128.0 + checksum), no runtime install; no GitLab token in code/logs/db/browser; runners never receive hosting credentials.
- Preserve review invariants (live refs, failed fetch fails preparation, frozen evidence, idempotent publication); zero GitHub regression; migrations append-only (none expected this phase).
- Rust files <= 2,500 lines (`pnpm check:file-size`); `publication.rs` must not grow.
- Conventional Commits, no AI attribution in commits/PRs, release note in `changelog.d/`.
- clippy `-D warnings`; errors must never embed credentials; GSD workflow for edits.

## Sources
- Primary: codebase (`backend/src/forge/mod.rs`, `server/publication.rs`, `server/review_http.rs`, `server/runtime.rs`, `server/status_comments.rs`, `server/queue_acknowledgements.rs`, `server/summary.rs`, `github/comments.rs`, `github/pr.rs`, `github/status_comments.rs`, `gitlab/api.rs`, `gitlab/mrs.rs`, `gitlab/read_tests.rs`).
- docs.gitlab.com/api/discussions (position parameters, line rules, versions) [CITED].
- GitLab CLI issues on `glab api --input` Content-Type (gitlab.com/gitlab-org/cli work_items 8316, 8517) via web search [MEDIUM].
- docs.gitlab.com/user/project/quick_actions (no statement about code fences) [CITED].
- `.planning/research/PITFALLS.md` (training-knowledge, MEDIUM).

## Metadata
**Confidence:** Standard stack HIGH (nothing new); architecture HIGH (verified against code); GitLab wire details MEDIUM (A1-A6).
**Research date:** 2026-11-05. **Valid until:** 30 days.
