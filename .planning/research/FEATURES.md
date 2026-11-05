# Feature Landscape

**Domain:** AI code review bot for self-hosted GitLab merge requests (fork of OpenCodeReview, polling, `glab` CLI)
**Researched:** 2026-11-05
**Confidence:** MEDIUM. GitLab REST API behavior is from training knowledge of the stable v4 API. Tool comparisons (Duo, CodeRabbit, PR-Agent) are from memory and were not re-verified online in this pass. Items marked (verify) need a check against the target instance's `/help` or API docs before the phase that uses them.

## How GitLab MR review bots work (baseline)

1. Discover open MRs (webhook in most tools; polling here via `GET /projects/:id/merge_requests?state=opened`).
2. Read the MR's `diff_refs` (`base_sha`, `start_sha`, `head_sha`) and `sha` (head).
3. Review the diff, then post:
   - Inline findings as **diff discussions**: `POST /projects/:id/merge_requests/:iid/discussions` with `body` and `position[position_type]=text`, `position[base_sha|start_sha|head_sha]`, `position[old_path|new_path]`, `position[new_line]` and/or `position[old_line]`.
   - One summary/status as a general **note**: `POST .../notes`, later edited with `PUT .../notes/:note_id`.
4. Never approve. Optionally set labels or an external commit status.

Positioning rules (HIGH on shape, verify edge cases): an added line takes only `new_line`; a removed line only `old_line`; an unchanged context line inside a hunk needs both. A line outside the diff hunks is rejected (HTTP 400, "line_code" error), so every inline post needs a fallback to a general note. A push after posting makes old discussions "outdated"; they are not deleted.

## GitHub -> GitLab capability mapping (this app)

| GitHub capability today | GitLab MR equivalent | Complexity | Notes |
|---|---|---|---|
| Polling discovery of open PRs | `GET /projects/:id/merge_requests?state=opened` (paginated, `updated_after` for cheap polling) | Low | `:id` may be the numeric id or the URL-encoded full path `group%2Fsub%2Fproject`. Store both numeric id and `path_with_namespace`. |
| PR number | MR **iid** (per-project) for every API path, `!iid` in UI. Global `id` is not used in paths | Low | Key rows by (project id, iid). Never use `id` for `/merge_requests/:iid`. Schema number column maps to iid. |
| Head SHA / new-head detection | MR `sha` field (head) and `diff_refs.head_sha` | Low | Compare to stored head. The list endpoint returns `sha`; `diff_refs` is more reliable from the single-MR GET (list may omit it or it may be lazily computed, verify). |
| Fetch of PR head without checkout | `git fetch origin +refs/merge-requests/<iid>/head:...` on the **target** project | Med | Works for same-project and fork MRs because GitLab writes the ref in the target repo. Base from live `origin/<target_branch>`; merge-base via git. Fail preparation on fetch error (invariant). `refs/merge-requests/<iid>/merge` is not needed and may not exist. |
| Fork PRs (head repo differs) | `source_project_id != target_project_id` | Low | Because the `head` ref lives in the target repo, no fork remote or fork credentials are needed. Posting uses target project only. |
| Inline findings (review comments on a line) | Diff discussion with `position[...]` (resolvable thread) | Med | Needs `base_sha/start_sha/head_sha` of the **reviewed** head. Re-read MR `diff_refs` immediately before posting; if head moved, abort publication (supersede). |
| Multi-line / side handling | `new_line`/`old_line` plus optional `line_range` | Med | Start with single-line anchor. Findings already anchor to changed lines (invariant), so mostly `new_line`. Removed-line findings need `old_line` + `old_path`. |
| Suggestions in summary / "Post suggestions" | Body of general note, or inline with a suggestion block | Low / Med | Keep the existing summary-section model. Inline ```` ```suggestion:-0+0 ```` is a differentiator, not required (see below). |
| Status comment | Single general note authored by the bot, edited in place | Low | `PUT .../notes/:note_id`. Find by `<!-- ocr-... -->` marker in note body, not by stored author alone. |
| Review summary | General note (same note or second note) | Low | Notes limit about 1,000,000 chars in recent versions (older instances 1,000,000; verify). Truncate with marker well below. |
| Publication idempotency | List existing discussions (`GET .../discussions`, paginate `per_page=100`), match `<!-- ocr-... -->` markers, skip or update | Med | GitLab has no client idempotency key. Idempotency = marker scan before post + DB record of posted discussion id + `PUT` for edits. Beware pagination and race with a second server instance (`run_exclusive`, `lock_pull_request`). |
| Superseding on new head | Mark old discussions resolved or leave them (GitLab auto-flags as outdated); update status note to "superseded by <sha>" | Med | Do not delete user-visible threads. Optional: resolve own outdated bot threads via `PUT .../discussions/:id` with `resolved=true`. |
| Repository access listing | `GET /groups` (with `all_available`/membership) and `GET /groups/:id/projects?include_subgroups=true`, `GET /projects?membership=true` | Med | "Workspace" = top-level group; subgroups are nested namespaces (`full_path` with slashes). Project identity is `path_with_namespace`, not `owner/repo`. Every place that splits `owner/name` on one slash breaks. Archived projects and projects without repository (disabled repo feature) must be filtered. |
| Indexing | `git fetch`/clone of project via HTTPS with `glab`-provided credential (git credential helper) + internal CA | Med | Largely host-agnostic once clone URL and CA trust work. Clone URL must come from `http_url_to_repo` (subgroup paths). |
| Repo link / PR link in UI | `web_url` of project and MR; "MR" and `!iid` wording | Low | Frontend terminology switch per deploy. |
| Auth (`gh`) | `glab auth login --hostname <host>`; `glab api` for REST calls | Med | `glab api` keeps tokens out of app code. Needs self-signed CA via `SSL_CERT_FILE`/system trust store, and `GIT_SSL_CAINFO` for git. |
| Draft PRs | MR `draft: true` (title prefix `Draft:`; `work_in_progress` deprecated) | Low | Decide policy (see table stakes). Filter via `draft` field, not title regex. |

## Table Stakes

Missing any of these and a GitLab user would consider the reviewer broken or noisy.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Inline findings as resolvable diff discussions | This is how humans and every bot comment on GitLab; threads block merge when "all threads must be resolved" is on | Med | Positions need correct SHA triple and path/line side. |
| Graceful fallback when an inline position is rejected | GitLab returns 400 for lines outside the diff, binary or renamed files, or moved head | Med | Fall back to a general note carrying the quote, marked with the same `<!-- ocr-... -->` marker. Never silently drop a validated finding. |
| Publication idempotency across retries and restarts | Duplicate bot threads are the number-one complaint | Med | Marker scan plus DB-recorded discussion ids; bound to the exact validated selection (existing invariant). |
| Single status note edited in place | Avoids comment spam on each push; PR-Agent and CodeRabbit both use a persistent/updated comment | Low | `PUT notes/:id`. Needs the bot to be the note author (token user owns it). |
| Review summary note (with suggestions section when enabled) | Parity with GitHub flow | Low | Reuse existing templates and markers; convert Markdown only where GitLab differs (see pitfalls). |
| New push triggers re-review and supersedes old head | Standard behavior | Med | Poll compares `sha`; cancel in-flight job for old head; update status note. |
| MR iid-keyed identity and `!iid` display | Wrong id breaks every API path | Low | Do not conflate with global `id`. |
| Namespaces: groups, subgroups, personal namespaces | Self-hosted installs nest groups deeply | Med | Use `path_with_namespace` and numeric project id; URL-encode in API paths. |
| Forks as MR source | Common in contributor workflows | Low | Via target repo `refs/merge-requests/<iid>/head`. Verify the reviewing account can read the target repo only. |
| Draft MR policy | Reviewing drafts wastes tokens and annoys authors; GitHub flow's behavior should be mirrored | Low | Default: mirror whatever the existing GitHub code does for draft PRs (check `polling_pr.rs`). Make it consistent, not new config (AGENTS.md says settings are fixed defaults). |
| Skip closed/merged/locked MRs; handle MR closed mid-review | Avoids posting to dead MRs | Low | Re-check `state` before publish. |
| Internal CA trust for REST and git | Stated target environment | Med | Both `glab` and `git`, in the server image only (runners never touch it). |
| Pagination everywhere (MRs, discussions, projects, groups) | GitLab defaults to 20 per page; missing pages creates dup threads and hidden projects | Low | Use `per_page=100` and follow `x-next-page`. |
| Rate-limit and 429/5xx handling with actionable errors | Self-hosted instances may have rate limits and slow proxies | Low | Respect `Retry-After`; surface actionable UI errors. |
| Never approve / never merge / never push | Product invariant | None | Do not request approval-API scopes; token needs `api` (or narrower via project/bot token with Developer role) only. |
| Actionable errors for insufficient permission | Posting needs Reporter+ (comment) role; suggestion apply needs Developer+ for humans | Low | Detect 403/404 (GitLab returns 404 for no-access) and explain. |

## Differentiators

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Inline ```` ```suggestion:-0+0 ```` blocks inside diff discussions | One-click "Apply suggestion" in GitLab UI, matches what CodeRabbit/PR-Agent/Duo provide | Med | Valid only in diff notes on the new side of a MR line; syntax `suggestion:-N+M` covers N lines above and M below the anchor (GitLab caps at 100 each, verify). Replacement must be exact code; fenced content inside suggestion block needs a longer outer fence if it contains backticks. Current product posts suggestions in the summary; moving them inline contradicts "listed in the review summary instead of posted inline" invariant, so treat as an explicit product decision, not a default. |
| Auto-resolve own threads when finding no longer applies on new head | Keeps MR clean | Med | Only for threads carrying our marker; never resolve human replies' threads. |
| Labels (for example `ocr::reviewed`, `ocr::needs-attention`) | Filtering and dashboards; scoped labels are native GitLab | Low | `PUT merge_requests/:iid` with `add_labels`. Needs Developer role. Product rules say posting config is fixed defaults; keep opt-in and tiny or defer. |
| External commit status or pipeline-visible check (`POST /projects/:id/statuses/:sha`) | Shows review state next to pipelines | Med | Risk: could be mistaken for a merge gate. Defer. |
| MR description or title awareness in review context (iid, labels, linked issues) | Better review quality | Med | Fits existing frozen evidence model; any added context must be frozen and validated. |
| UI deep links to the discussion (`web_url#note_<id>`) | Quick navigation | Low | Store discussion/note id on publication. |
| Reviewer assignment as trigger (like `@GitLabDuo` reviewer) | Opt-in review by assigning a bot user | Med | Works with polling (`reviewer_username=` filter). Useful to cut cost on big instances. Not required for parity. |
| Batch review of all open MRs in selected groups | Matches "Repository access" selection | Med | Same as GitHub selection model, with group-level expansion. |

## Anti-Features

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Webhooks, system hooks, GitLab CI-component integration | Project decision (polling); would require exposing server | Polling with `updated_after`. |
| Approving MRs, unapproving, or merge-when-pipeline-succeeds | Product invariant: nothing approves automatically | Comments only. |
| Slash commands and chat replies (`/review`, `@coderabbitai`, `/improve`) | Interactive chat needs webhooks and a new product area | Re-review happens on new head or UI action. |
| Managed GitLab tokens, OAuth app in OpenCodeReview | Rule: auth through `glab` | `glab auth login`. |
| Multiple GitLab instances or GitHub + GitLab in one deploy | Out of scope | One provider, one host per deploy. |
| Checking out the user's branch or cloning fork repos | Invariant: never check out; fork head already in target refs | `git fetch` refs and `git diff`/`show`. |
| Posting MR description rewrites (PR-Agent `/describe`), auto-title, auto-labeling by AI | Mutates author content; outside reviews/indexing/skills | Leave description untouched. |
| Cost or currency display | Product rule: tokens only | Tokens only. |
| Creating issues, pushing commits, "apply fix" automation | Out of product scope | Suggestions only. |
| Letting runners reach GitLab | Sandbox invariant; egress allows AI hosts only | All GitLab I/O stays in `server`. |
| Duplicating state per GitLab "version" (MR diff versions API) as a second source of truth | Adds complexity; frozen archive plus live SHAs suffice | Use `diff_refs` only for the position triple. |

## What comparable tools do on GitLab (context, MEDIUM/LOW confidence)

| Tool | Trigger | Output | Takeaway |
|------|---------|--------|----------|
| GitLab Duo Code Review (Premium/Ultimate) | Assign Duo as reviewer or mention it; newer agentic flow | Inline discussions with suggestions, summary; custom review instructions in repo | Native UI integration; inline suggestions are the norm. Not available for free/self-managed without the license, which is why this fork exists. |
| CodeRabbit | Webhook, supports self-managed GitLab | Walkthrough note updated in place, inline comments, committable suggestions, incremental review on new commits, chat | Persistent edited summary note and incremental review are the expected UX. |
| PR-Agent / Qodo Merge (open source) | Webhook server or CI; slash commands | Persistent comment updated in place (`/review`), `/improve` posts suggestion blocks, configurable TLS verification for self-hosted | Persistent-comment-with-marker pattern identical to what OCR already does with `<!-- ocr-... -->`; supports custom CA. |

## Feature Dependencies

```
Provider abstraction (host trait) -> every feature below
glab auth + CA trust -> MR list / fetch / publish / repository listing / indexing
Project identity (id + path_with_namespace + iid) -> polling, fetch refspec, publish, UI links
MR polling (sha change) -> review preparation (refs/merge-requests/<iid>/head) -> evidence freeze -> publication
diff_refs read at publish time -> inline discussion position
Inline discussion -> general-note fallback
Marker scan + DB discussion/note ids -> idempotency -> status note edit-in-place -> superseding
Inline discussion -> (differentiator) suggestion blocks, auto-resolve
Group/subgroup listing -> Repository access selection -> polling scope
```

## MVP Recommendation

Prioritize (phase order):
1. Provider layer, `glab` auth, CA trust, project identity (id, `path_with_namespace`, iid).
2. Repository access listing (groups, subgroups, projects) and MR polling with new-head detection, draft handling mirroring GitHub.
3. Preparation via `refs/merge-requests/<iid>/head` (covers forks), live base and merge-base.
4. Publication: inline diff discussions with fallback to notes, marker-based idempotency, status note edited in place, summary note, superseding.
5. Indexing for GitLab projects, UI terminology (MR, `!iid`, `web_url`).

Defer:
- Inline suggestion blocks: conflicts with the current "suggestions live in the summary" invariant; decide explicitly later.
- Labels, commit statuses, auto-resolve, reviewer-assignment trigger: nice-to-have, none needed for the "real MR reviewed with inline findings" done criterion.

## Sources

- Training knowledge of GitLab REST API v4 (Discussions, Notes, Merge Requests, Projects, Groups APIs) - MEDIUM; not re-verified online in this pass. Confirm against `https://docs.gitlab.com/api/discussions/` and `https://docs.gitlab.com/user/project/merge_requests/reviews/suggestions/` for the target GitLab version.
- Training knowledge of GitLab Duo Code Review, CodeRabbit and PR-Agent behavior - LOW to MEDIUM; verify before citing in user docs.
- Project files: `.planning/PROJECT.md`, `.planning/codebase/ARCHITECTURE.md`, `AGENTS.md`.
