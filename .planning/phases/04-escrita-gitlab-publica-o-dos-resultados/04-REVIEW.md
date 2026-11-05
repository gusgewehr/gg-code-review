---
phase: 04-escrita-gitlab-publica-o-dos-resultados
reviewed: 2026-11-05T00:00:00Z
depth: standard
iteration: 2
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
  critical: 0
  warning: 3
  info: 8
  total: 11
status: issues_found
---

# Phase 4: Code Review Report (re-review after fixes)

**Reviewed:** 2026-11-05
**Depth:** standard
**Files Reviewed:** 22 (focus: `git diff 86fa332..HEAD`, fix commits 2cc0f72..6039326)
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

This pass checked the six fixes from the first review (CR-01, CR-02, WR-01..WR-04) for correctness and regressions.

**Verified fixed:**
- **CR-02.** The part marker `<!-- ocr-review-part:v1 run=<id> head=<sha> -->` is built the same way on both sides:
  - Publisher side: `run=` token of `review_post_marker`, plus `payload.commit_id`, which is `run.head_sha` (`publication.rs:1246`).
  - Dedupe side: `run.id` plus `run.head_sha` (`publication.rs:1326`).
  - The marker sits on its own line, so the sanitizer leaves it alone, and the fallback and PUT bodies carry it too.
  - On a retry, this run's own notes are left out of `existing_ocr_comments`, so `only_duplicates` stays false and the summary is built from the full list. Inline writes that already landed are still skipped by fingerprint in `post_review_at`. Publication stays idempotent and posts no duplicate notes.
- **WR-01.** `check_head` now runs before every inline write.
- **WR-02.** `latest_version` polls `/versions` 4 times, waiting 1, 2 and 4 s, and honours cancel. The message no longer promises a retry.
- **WR-03.** The status is read only from stderr, and only from glab's `HTTP NNN` suffix or a status that leads a line. Stdout and ids never count. A POST that gets a 5xx is still not retried.
- **WR-04.** `own_note_with_marker` filters by `GET /user`, both in `find_review_id_by_marker_at`/`upsert` and in the publisher's `existing_summary`. If `/user` fails, nothing is written (fail-closed). If the failure happens during reconcile, the run stays locked.

**Invariants hold:**
- **GitHub flow unchanged.** `forge::dedupe_review_comments` drops `run_id`/`head_sha` on the GitHub arm, and it has a single caller.
- **No token exposure.** Status parsing never logs stderr, and the log line prints only the `classify` code.
- **Never approves or merges.** `payload["event"]` is still never read.
- **Fail-closed.** `list_notes` failures abort before any write.

**CR-01 is mostly fixed, with gaps.**
- Closed: the five bypasses from the first review, the `@all` mention and the quick-action vectors.
- Still open: two gaps in the "no applicable suggestions / no references" promise (WR-01, WR-03 below).
- New: a link-corruption regression introduced by the narrower URL exemption (WR-02 below).

None of these reaches BLOCKER level: none sends a notification or performs a write action.

## Warnings

### WR-01: `suggestion` fences nested in a blockquote or list item still produce applicable GitLab suggestions

**File:** `backend/src/gitlab/sanitize.rs:30-40, 128-135`
**Issue:** `defuse_suggestion` relies on `fence_of`, which only recognises a fence that starts the line, after up to 3 spaces. CommonMark also opens fenced code blocks inside containers, so these inputs still render a `suggestion` code block on a DiffNote:
- `> ```suggestion`
- `- ```suggestion` / `1. ```suggestion`
- A fence indented 4 or more spaces under a list item, which `fence_of` rejects because `indent > 3`

GitLab finds suggestion blocks anywhere in the rendered note. A prompt-injected finding can therefore offer a one-click "Apply suggestion" that writes attacker-chosen code. The module doc (line 1) and the fix report say suggestions are defused "whatever the fence state", but they are not. This gap existed before the fix, but CR-01's fix explicitly claims to cover it.
**Fix:** Defuse the info string wherever a fence run appears in the line, not only at the line start:
```rust
fn defuse_suggestion(line: String) -> String {
    // Any ``` or ~~~ run (in a blockquote, list item or deep indent) followed by `suggestion` becomes `diff`.
    let mut out = String::with_capacity(line.len());
    let mut rest = line.as_str();
    while let Some(i) = rest.find(|c| c == '`' || c == '~') {
        let ch = rest[i..].chars().next().unwrap();
        let run = rest[i..].chars().take_while(|c| *c == ch).count();
        out.push_str(&rest[..i + run]);
        rest = &rest[i + run..];
        if run >= 3 && rest.trim_start().starts_with("suggestion") {
            out.push_str("diff");
            rest = "";
        }
    }
    out.push_str(rest);
    out
}
```
Add tests for `> ```suggestion`, `- ```suggestion` and `-   \n    ```suggestion`.

### WR-02: The narrower URL exemption now writes zero-width spaces into valid links, which breaks them (regression)

**File:** `backend/src/gitlab/sanitize.rs:10-12, 54, 95`
**Issue:** `URL_END` ends the exempt span at `_`, `~`, `*`, `!`, `$`, `^`, `'` and `,`. GitLab, and GFM autolinking in general, keeps these characters inside an autolink when they are not trailing. Any sigil after one of them in the same URL is then treated as live, and a ZWSP is written inside the link, so the `href` changes. Before the fix, a `://` token was exempt as a whole, so this is new breakage. Examples:
- `https://gl.example/acme_web/app/-/blob/abc/docs/Read%20Me.md` becomes `...Read%​20Me.md`. `%2` counts as a live milestone sigil because `2` is a word character.
- `https://gl.example/acme_web/ui/-/tree/main/packages/@ui/button` becomes `.../@​ui/...`.
- `https://h/x?a=1&2=b` and `https://h/a!1` get similar damage.

Underscores are common in GitLab group and project paths, so model-written links to files with spaces, or to scoped-package directories, are broken in published notes. OCR's own commit links (`/-/commit/<hex>`) are not affected.
**Fix:** Apply the strict end set only to the host part, and end the path part only at characters that really end an autolink:
- Host part: refuse the exemption when the host contains `_` or a character that is not valid in a host.
- Path part: end the span only at `<`, `` ` ``, `"`, `'`, `)`, `]`, `|` and `\`.
- Trailing `?!.,:*_~` at the very end of the token can still be trimmed from the span.

Add regression cases: `https://h/a_b/c%20d`, `https://h/a_b/@scope/x`, and keep `[x](https://h)@all`, `xhttps://h/@all` and `https://a/b,@alice` neutralised.

### WR-03: Commit and URL references still get through, so the "no references" promise is still false

**File:** `backend/src/gitlab/sanitize.rs:1, 73, 95`; `changelog.d/feat-gitlab-connection.md:17`
**Issue:**
- **Commit references.** The `@` rule skips any `@` preceded by an alphanumeric character, which keeps emails intact. That same rule leaves GitLab's cross-project commit reference `group/project@<7-40 hex>` (and `@sha...sha` ranges) live. Bare 7-40 hex SHAs are not touched either.
- **URL references.** An exempt `http(s)` URL pointing to an issue, MR or commit on the same instance is also turned into a reference by GitLab's link-reference filters.
- **Effect.** Both forms render as references and create "mentioned in" system notes on the target issue, MR or commit.

Prompt-injected text can therefore still spam cross-reference notes onto arbitrary readable issues, MRs or commits. The module doc (line 1: "no mentions or cross references") and the changelog (line 17: "cannot trigger quick actions, mentions or references") promise otherwise. The mention and quick-action vectors are closed, so this is a WARNING, not a BLOCKER.
**Fix:** Neutralise the `@` in `<path>@<hex{7,}>`, so that an `@` preceded by an alphanumeric character and followed by 7 or more hex characters gets a ZWSP. Emails rarely have a 7-character hex local domain, so they survive. Reword the doc and the changelog to the guarantee the code actually gives, for example: "model text cannot trigger quick actions or mentions, and short-form references are neutralised". Full instance URLs to issues or MRs cannot be defused without breaking links.

## Info

### IN-01: Leftover from CR-02 (deferred): Skip or Mark reviewed after a partial GitLab publication leaves local state saying nothing was published

**File:** `backend/src/server/publication.rs:1012-1030`
**Issue:** The fix report lists this as not addressed because it needs a DB/state change. Some inline findings can be live on the MR while the run is reset to awaiting action. Carried forward so it stays visible.
**Fix:** Track the number of inline writes per run, or warn in the UI before Skip/Mark reviewed when this run's part marker exists on the MR.

### IN-02: An aborted inline loop leaves discussions with no summary, and later runs treat them as another run's duplicates

**File:** `backend/src/gitlab/publish.rs:163`
**Issue:** When `check_head` fails mid-loop, which is now more likely, the run is reset and its notes stay without a summary. The run for the new head suppresses matching findings as "already posted by another OpenCodeReview run". If every finding matches, it posts no summary, so the MR ends up with findings and no summary anywhere. This cannot happen on GitHub, where the review is posted atomically. It is not new: before the fix, the loop always ran to the end and only the summary was aborted.
**Fix:** Optionally, have `dedupe_review_comments` count only notes from runs whose summary marker exists.

### IN-03: `http_status` accepts three digits followed by a letter

**File:** `backend/src/gitlab/write.rs:29-32`
**Issue:** `code` takes leading ASCII digits and does not require a boundary after them. A stderr line starting with `409c2e...` or `400abc` parses as 409/400, which means Rejected and therefore a fallback note. This is unlikely with real glab output.
**Fix:** Require that the next character is not alphanumeric: `s[3..].chars().next().is_none_or(|c| !c.is_ascii_alphanumeric())`.

### IN-04 (carried, was IN-01): `queued_commit_url` duplicates `Forge::commit_url`, and the text says "pull request" on GitLab

**File:** `backend/src/server/comment_activation.rs:7-12, 18`
**Fix:** Reuse `forge::current().commit_url(...)`, and pick the "merge request" or "pull request" wording from the forge.

### IN-05 (carried, was IN-02): `diffs_at` hard-codes the backoff and rebuilds the MR endpoint

**File:** `backend/src/gitlab/diffs.rs:361, 369`
**Fix:** Pass `&Ctx` (or `backoff_ms`) and use `mrs::mr_endpoint`.

### IN-06 (carried, was IN-03): `forge::GitHubStatusComments` also dispatches to GitLab

**File:** `backend/src/forge/mod.rs:226-227`
**Fix:** Rename it to `ForgeStatusComments`.

### IN-07 (carried, was IN-04): GitLab fallback notes have no path or line, so cross-push dedupe does not match them

**File:** `backend/src/gitlab/notes.rs:258-268`
**Fix:** Parse the `` `path:line` `` prefix that `fallback_body` writes, and use it for general notes.

### IN-08 (carried, was IN-05): `"diff": null` fails the whole diff parse, and `/diffs` needs GitLab 15.7 or later

**File:** `backend/src/gitlab/diffs.rs:270-274, 361`
**Fix:** Use `diff: Option<String>` with `unwrap_or_default()`, and document the minimum GitLab version.

---

_Reviewed: 2026-11-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
