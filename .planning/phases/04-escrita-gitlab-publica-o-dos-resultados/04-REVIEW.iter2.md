---
phase: 04-escrita-gitlab-publica-o-dos-resultados
reviewed: 2026-11-05T00:00:00Z
depth: standard
files_reviewed: 22
files_reviewed_list:
  - backend/src/forge/mod.rs
  - backend/src/github/comments.rs
  - backend/src/github/mod.rs
  - backend/src/github/status_comments.rs
  - backend/src/gitlab/diffs.rs
  - backend/src/gitlab/mod.rs
  - backend/src/gitlab/mrs.rs
  - backend/src/gitlab/notes.rs
  - backend/src/gitlab/publish.rs
  - backend/src/gitlab/publish_tests.rs
  - backend/src/gitlab/sanitize.rs
  - backend/src/gitlab/test_fake.rs
  - backend/src/gitlab/write.rs
  - backend/src/review/findings.rs
  - backend/src/server/comment_activation.rs
  - backend/src/server/publication.rs
  - backend/src/server/queue_acknowledgements.rs
  - backend/src/server/review_http.rs
  - backend/src/server/status_comments.rs
  - backend/src/server/status_comments_test_support.rs
  - backend/src/server/status_comments_tests.rs
  - changelog.d/feat-gitlab-connection.md
findings:
  critical: 2
  warning: 4
  info: 5
  total: 11
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-11-05
**Depth:** standard
**Files Reviewed:** 22
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

Reviewed the phase-4 diff (`eaaffee..HEAD`): the forge dispatch seam, the GitLab write layer (`write.rs`), notes and status notes (`notes.rs`), the diff line map (`diffs.rs`), the publisher (`publish.rs`), the body sanitizer (`sanitize.rs`), and the server changes that turn on GitLab publication and acknowledgements.

What holds up:
- No token reaches argv, logs or error strings. Bodies go through a 0600 `--input` file, and error text is fixed or redacted.
- No code path approves, requests changes on or merges an MR. `payload["event"]` is never read, and `approve_post`/`request_changes` are rejected over HTTP.
- The GitHub flow does not regress. `StatusCommentApi::edit` defaults to `Ok(false)`, so GitHub keeps post-then-delete. The `delete` signature change is mechanical.
- Retrying a run never posts a second inline discussion. Fingerprint dedupe against `list_notes` fails closed.

What is wrong:
1. The sanitizer that should stop model text from triggering mentions and references can be bypassed in several easy ways.
2. GitLab publication is not atomic. When it fails partway, the GitHub-shaped dedupe in `finish_review_post` on the retry drops the summary note for good, or posts a summary that counts only the remaining findings.
3. Smaller problems: the head check does not cover the inline loop, an error message promises a retry that never happens, HTTP status detection by token matching is fragile, and marker lookups match notes from any author.

## Critical Issues

### CR-01: The sanitizer can be bypassed: model text can still @mention users (including `@all`) and create cross-references

**File:** `backend/src/gitlab/sanitize.rs:13-15, 30-45, 61-74, 89-93`
**Issue:** The changelog (`changelog.d/feat-gitlab-connection.md:715`) and the module doc promise that model text "cannot trigger quick actions, mentions or references". Model text comes from a prompt that includes attacker-controlled MR content, and it gets around the sanitizer in at least five ways:

1. **Marker-line exemption.** Any line that starts with `<!--` is copied verbatim (lines 90-93). `<!-- -->@all please look` renders as a live `@all` mention, because GitLab strips the comment and keeps the rest of the line.
2. **Unbalanced backticks.** `neutralize_line` splits on `` ` `` and treats every odd segment as code (lines 63-71). With an unmatched backtick, as in ``Use ` to quote @all``, the backtick renders literally, but ` to quote @all` is skipped, so the mention fires. A code span that opens on one line and closes on the next has the same flaw: everything after the closing backtick on the second line is skipped.
3. **URL tokens.** Any whitespace-delimited token that contains `://` is copied untouched (lines 31-34). `[docs](https://x.y)@all` and `https://a/b,@alice` keep their mentions, because GitLab only needs a non-word character before `@`.
4. **Cross-project references.** A sigil right after an alphanumeric character is never neutralized (line 39), so `group/project#12` and `project!3` render as references. They create "mentioned in" system notes on the target issue or MR. Quoted label and milestone references (`~"bug fix"`, `%"v1.0"`) also pass, because the next character is `"`.
5. **Fence-state drift.** An open fence in the sanitizer skips neutralization (line 95). Any construct where the sanitizer thinks a fence is open but GitLab does not, such as ```` ``` ```` inside an HTML block like `<div>`, leaves the following lines live.

This breaks the phase's stated security control against prompt-injected output.

**Fix:** Do not exempt whole lines or tokens. Neutralize every non-code text run, and treat only spans GitLab itself treats as code as code:
```rust
// 1) marker exemption: only lines that are exactly an OCR marker
fn is_marker(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("<!-- ocr") && t.ends_with("-->") && t.matches("-->").count() == 1
}
// 2) inline code: only balanced backtick runs (same length open/close) on the line; an
//    unmatched backtick is plain text and its segment must be neutralized
// 3) URLs: neutralize sigils that follow a non-URL char even inside a "://" token
//    (or neutralize every sigil after ')' ',' etc.), keep only the URL span itself
// 4) insert ZWSP before '#', '!', '&', '%', '~', '$' when followed by an alnum or '"',
//    regardless of the preceding character (covers `group/project#12`, `~"label"`)
```
Add regression tests for each of these inputs: `<!-- -->@all`, ``a ` @all``, `[x](https://h)@all`, `g/p#1`, `~"x y"`.

### CR-02: A partial GitLab publication permanently loses the summary note (or posts a wrong one) and records misleading state

**File:** `backend/src/gitlab/publish.rs:103-139` (interacting with `backend/src/server/publication.rs:1370-1372, 1335-1337, 1012, 1063-1067`)
**Issue:** GitHub creates the review in one atomic call. `post_review_at` instead posts N inline discussions and then the summary note last. POST 5xx responses are never retried (`write.rs:247`). If the summary POST (or any later inline POST) fails, `publish_review_with_recovery` finds no summary marker, calls `reset_review_posting`, and the run goes back to awaiting action, even though some or all findings are already on the MR. On the retry, whether automatic, manual Comment, or startup reconcile, `compose_review_post` dedupes against the notes now on GitLab:
- **All inline comments were posted before the failure:** every comment is suppressed, so `only_duplicates` is true and the body is empty (`publication.rs:1370-1372`). `should_post` becomes false (`1335-1337`), so the summary note (including suggestions when Post suggestions is on) is never posted. The run is finalized with "Findings already posted by another OpenCodeReview run" (`1063-1067`), which is false: they were posted by this run.
- **Some inline comments were posted:** the generated summary table (`posted_findings_review_body`) is built from the remaining comments only, so the counts and rows under-report the review.
- **Meanwhile, if the user picks Skip or Mark reviewed** after the failure, local state says nothing was published while findings sit on the MR.

**Fix:** Make the retry aware of this run's own partial publication. When an inline comment is suppressed because a note from this run already carries its fingerprint, keep it in the summary input, and post the summary whenever this run's summary marker is missing. For example, have `post_review_at` receive the full validated comment list (pre-dedupe) for the summary and dedupe only for the inline writes. Alternatively, build the summary from `annotated_comments_json` (statuses `new` + `duplicate_suppressed`) instead of `post_comments`. At minimum, on GitLab do not treat `only_duplicates` as "nothing to post" while `find_review_id_by_marker(post_marker)` returns `None`.

## Warnings

### WR-01: The head check does not cover the inline loop, so findings are posted against a superseded head

**File:** `backend/src/gitlab/publish.rs:81, 103-131, 137`
**Issue:** `check_head` runs once before the loop and once before the summary. A push during the loop, which can be long with N sequential `glab` calls each allowed up to 120 s, lets every remaining inline discussion post against the old `head_sha`. GitLab accepts these as outdated diff notes. Only the summary is aborted. The changelog says "a push after the review aborts publication" (`changelog.d/feat-gitlab-connection.md:714`), which is not what happens.
**Fix:** Re-run `check_head` (or at least a cheap `mr.sha` fetch) before each inline write, or every few writes, and stop as soon as it changes. Otherwise, reword the changelog to say that a push aborts the summary, not the inline comments.

### WR-02: The PENDING error promises a retry that never happens

**File:** `backend/src/gitlab/publish.rs:14-15, 90-92`
**Issue:** When `versions[0].head_commit_sha` lags behind the head, publication fails with "...publication will be retried." Nothing retries it. `handle_completed_review` (`publication.rs:743-759`) sets the lane to `ready` with "Auto-post failed: ...", and `publish_review_with_recovery` resets the run. The user is told to wait for something that never comes.
**Fix:** Either retry in place (poll `/versions` a few times with backoff before failing; GitLab usually catches up within seconds), or change the text to an actionable one: "GitLab has not finished processing the latest push; publish again in a moment."

### WR-03: HTTP status detection by token/substring match on combined stdout+stderr can misclassify errors and cause duplicate notes

**File:** `backend/src/gitlab/write.rs:177-180, 228-247` (with `gitlab/auth.rs:106-108`)
**Issue:** `text` is `text_of(&out)`, which is stdout plus stderr and includes the response body. `classify` treats any substring `401` as unauthorized: a note id, project id or iid containing `401` turns into "token invalid". `has_status` matches any standalone `400`/`404`/`409`/`422`/`5xx` token. If glab echoes the request URL or path in its error output, an MR with iid 400, 409 or 422 turns any failure, including a 5xx where the write may have landed, into `Rejected`. `post_comment` (`publish.rs:67`) then posts a fallback general note, which duplicates a discussion that may already exist. Likewise, iid 404 turns 5xx errors into `NotFound`, and `delete_note` treats `NotFound` as success.
**Fix:** Parse the status explicitly instead of scanning free text. For example, use `glab api -i`/`--include`, read the `HTTP/x NNN` status line from stdout, and classify on that number alone. If that is not possible, match only the `(HTTP NNN)` suffix glab prints on stderr, and never scan stdout.

### WR-04: Marker and summary lookups match notes from any author

**File:** `backend/src/gitlab/notes.rs:196-206, 219-222`; `backend/src/gitlab/publish.rs:99-100`
**Issue:** `find_review_id_by_marker_at`, `upsert_issue_comment_by_marker_at` and `existing_summary` accept the first note containing the marker, whoever wrote it. The PR-summary marker is static and predictable (`pr_summary_marker(full_name, number)`). Any participant who posts a note containing it makes `upsert` try to PUT someone else's note, which fails with 403 on every attempt, so the PR summary is permanently broken for that MR. Review markers are visible in the note source of earlier runs. A planted marker for a future run makes `existing_summary` skip the summary, and makes reconciliation accept a note OpenCodeReview never wrote as its review id. Status comments already filter by `publisher_id`; these lookups do not. The GitHub counterpart has the same gap, but on GitLab the consequence is a write to a foreign note.
**Fix:** Filter by the authenticated user in every GitLab marker lookup:
```rust
let me = GitLabStatusComments(ctx.clone()).publisher_id()?; // or pass it in
list_notes(ctx, full_name, number)?.into_iter()
    .filter(|n| n.author.id == me)
    .find(|n| n.body.contains(marker))
```

## Info

### IN-01: `queued_commit_url` duplicates `Forge::commit_url`, and the text still says "pull request" on GitLab

**File:** `backend/src/server/comment_activation.rs:7-12, 18`
**Issue:** `Forge::commit_url` / `gitlab_commit_url` (`forge/mod.rs:301-305`) already build this URL. The acknowledgement heading reads "OpenCodeReview is reviewing this pull request" on a merge request.
**Fix:** Reuse `forge::current().commit_url(...)`, or fix the GitHub branch of `commit_url` to use `gh_host()` and reuse that. Pick "merge request" or "pull request" from the forge.

### IN-02: `diffs_at` hard-codes the backoff and rebuilds the MR endpoint

**File:** `backend/src/gitlab/diffs.rs:361, 369`
**Issue:** It builds the endpoint with `format!` instead of `mrs::mr_endpoint`, and it ignores `Ctx::backoff_ms` (fixed at 1000). The publisher's injected backoff therefore does not apply to this call.
**Fix:** Take `&Ctx` (or `backoff_ms`) and use `super::mrs::mr_endpoint(full_name, number)`.

### IN-03: `forge::GitHubStatusComments` now dispatches to GitLab

**File:** `backend/src/forge/mod.rs:226-227`
**Issue:** The name says GitHub, but the type also dispatches to GitLab, which makes call sites misleading.
**Fix:** Rename it to `ForgeStatusComments` (four call sites).

### IN-04: GitLab fallback notes have no path or line, so the cross-push dedupe does not match them

**File:** `backend/src/gitlab/notes.rs:242-251`
**Issue:** General notes, meaning fallback findings and the summary that carries relocated findings, map to `path: ""` and `line: 0`, so `already_posted`'s "same title in the same file" rule never matches them. Matching falls back to the fingerprint, which includes the line, so a finding first posted as a fallback note is posted again after a push that shifts its line.
**Fix:** Parse the `` `path:line` `` prefix that `fallback_body` writes, and use it as the path and line for general notes.

### IN-05: Diff parsing fails as a whole on `"diff": null`, and `/diffs` needs GitLab 15.7 or later

**File:** `backend/src/gitlab/diffs.rs:270-274, 361`
**Issue:** `#[serde(default)] diff: String` handles a missing field but not `null`, so one such entry fails the whole `parse_pages` call. Every publication would fail, and anchor checks would be silently skipped. `GET /merge_requests/:iid/diffs` also does not exist before GitLab 15.7, so publication always fails on older self-hosted instances, and nothing documents that.
**Fix:** Use `diff: Option<String>` with `unwrap_or_default()`. Document the minimum GitLab version, or fall back to `/changes`.

---

_Reviewed: 2026-11-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
