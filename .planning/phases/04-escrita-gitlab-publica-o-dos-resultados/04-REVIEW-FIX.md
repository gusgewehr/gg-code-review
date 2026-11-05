---
phase: 04-escrita-gitlab-publica-o-dos-resultados
fixed_at: 2026-11-05T00:00:00Z
review_path: .planning/phases/04-escrita-gitlab-publica-o-dos-resultados/04-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 4: Code Review Fix Report

**Fixed at:** 2026-11-05
**Source review:** .planning/phases/04-escrita-gitlab-publica-o-dos-resultados/04-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6 (2 critical, 4 warning; Info excluded by `critical_warning` scope)
- Fixed: 6
- Skipped: 0

Verification: `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --locked -D warnings` pass. The filtered test runs all pass: `--lib sanitize` 18, `--lib gitlab` 74, `--lib publication` 47 (1 ignored), `--lib status_comments` 18, `--lib forge` 8.

## Fixed Issues

### CR-01: The sanitizer can be bypassed: model text can still @mention users (including `@all`) and create cross-references

**Files modified:** `backend/src/gitlab/sanitize.rs`
**Commit:** 2cc0f72
**Status:** fixed: requires human verification
**Applied fix:**
- Only a line that is exactly one well-formed HTML comment is exempt now. That means `<!--`...`-->` with no `--` inside and no `<!-->` or `<!--->` forms. `<!-- -->@all` and `<!-- a --!> @all -->` are neutralised.
- Inline code and fences are no longer exempt. Every non-marker line is neutralised, because the sanitizer cannot reliably match what GitLab renders as code (unbalanced backticks, code spans across lines, HTML blocks, list containers). This closes bypasses 2 and 5 completely instead of modelling CommonMark. A `suggestion` fence opener becomes `diff` whatever the fence state.
- URL exemption: only `http(s)://` links not glued to preceding text are exempt, and only up to the first character that can end a link (`)`, `]`, `,`, `>`, `` ` ``, `*`, `_`, `|`, etc.). `[x](https://h)@all`, `https://a/b,@alice` and `xhttps://h/@all` are neutralised.
- Sigil rules now follow GitLab's reference grammar, with no condition on the preceding character, except for `@`, which keeps it so emails survive:
  - `#`, `!` and `$` followed by a digit.
  - `&` followed by a digit or starting an HTML entity (`&commat;`, `&#x40;`).
  - `~` and `%` followed by a word character or `"`.
- This covers `g/p#1`, `project!3`, `~"x y"` and `%"v1.0"`. Code-heavy text such as `&mut`, `#include` and `!flag` stays clean.
- Added regression tests for each bypass in the review, plus HTML-entity and HTML-block cases.

Trade-off: invisible zero-width spaces can now appear inside code blocks, after `@word`, `%d`, `~Foo`, `$1`, `#1` and `&1`. Code copied from a GitLab note can carry them. Human check: confirm that cost is acceptable, and that the "exact comment line" rule matches every OCR marker format. All current markers are single-line comments with no `--` inside.

### CR-02: A partial GitLab publication permanently loses the summary note (or posts a wrong one) and records misleading state

**Files modified:** `backend/src/gitlab/publish.rs`, `backend/src/gitlab/notes.rs`, `backend/src/forge/mod.rs`, `backend/src/server/publication.rs`, `backend/src/gitlab/publish_tests.rs`
**Commit:** ad22fcf
**Status:** fixed: requires human verification
**Applied fix:**
- Every inline note the GitLab publisher writes now ends with a per-run marker: `<!-- ocr-review-part:v1 run=<id> head=<sha> -->`. The run id comes from the `run=` token of the summary marker.
- `forge::dedupe_review_comments` now takes `run_id` and `head_sha`. GitHub ignores them, so the GitHub path is unchanged. On GitLab, `existing_ocr_comments` leaves out notes that carry this run's part marker.
- On a retry after a partial failure, this run's own findings stay in `post_comments`:
  - The summary is built from the full list.
  - `only_duplicates` and the "already posted by another OpenCodeReview run" reason apply only to other runs' notes.
  - The summary note is posted when its marker is missing.
- The publisher still skips the inline writes that already landed, by fingerprint. Publication stays idempotent.
- Added a test: this run's finding 1 is already posted and another run's finding 3 is already posted. Result: only finding 3 is suppressed, finding 2 is posted with the part marker, and the summary is posted.

Not addressed (needs a DB/state change, out of scope for a minimal fix): if the user picks Skip or Mark reviewed after a partial failure, local state still says nothing was published.

### WR-01: The head check does not cover the inline loop, so findings are posted against a superseded head

**Files modified:** `backend/src/gitlab/publish.rs`, `backend/src/gitlab/publish_tests.rs`
**Commit:** af2bf6d
**Applied fix:** `check_head` now runs before every inline write, after the fingerprint skip, as well as before the summary. A push mid-loop stops the remaining discussions. The race test now covers three comments, with the head changing before the second write: one write happens and no summary is posted. Cost: two extra GETs per inline write (MR plus target-branch tip, through `live_state_at`).

### WR-02: The PENDING error promises a retry that never happens

**Files modified:** `backend/src/gitlab/publish.rs`, `backend/src/gitlab/publish_tests.rs`
**Commit:** 13da6ca
**Applied fix:** A new `latest_version` helper polls `/versions` up to 4 times. The waits use `Ctx::backoff_ms` (1s, 2s, 4s live) and the loop honours cancel. If the version never catches up, the error now reads "...publish again in a moment." Added a test where versions catch up on the 3rd poll and publication succeeds, plus a message assertion for the lagging case.

### WR-03: HTTP status detection by token/substring match on combined stdout+stderr can misclassify errors and cause duplicate notes

**Files modified:** `backend/src/gitlab/write.rs`
**Commit:** 5541d15
**Status:** fixed: requires human verification
**Applied fix:**
- `has_status` (which scanned stdout and stderr together) and the classify-based 401 check are replaced by `http_status(stderr)`. It reads glab's `(HTTP NNN)` or `HTTP NNN` suffix. If there is none, it accepts only a 3-digit status leading a stderr line, after an optional `glab:` or `ERROR:` prefix.
- Stdout is never scanned, and ids or paths in the middle of a line never count.
- Classification uses the number alone: 401 means a token error, 403 a scope error, 400/409/422 Rejected, 404 NotFound, and 429 (plus 5xx for non-POST) is retried. `classify` is still used, but only for the log code.
- Added tests for glab-format suffixes and for a 5xx on MR iid 400 with `401`/`400` ids in stdout: it classifies as Other, not Rejected and not a token error. A network-error line holding `/merge_requests/409/` yields no status.

Human check: glab 1.128.0's exact error format is still unconfirmed (assumption A4 in 04-RESEARCH.md). The leading-status fallback keeps the existing fake formats working. If real glab output matches neither form, write errors degrade safely to Other: no fallback note and no retry.

### WR-04: Marker and summary lookups match notes from any author

**Files modified:** `backend/src/gitlab/notes.rs`, `backend/src/gitlab/publish.rs`, `backend/src/gitlab/publish_tests.rs`
**Commit:** 6039326
**Applied fix:**
- Added `current_user_id(ctx)`, which `publisher_id` now uses too, and `own_note_with_marker(notes, me, marker)`.
- `find_review_id_by_marker_at`, which `upsert_issue_comment_by_marker_at` also uses, and the publisher's `existing_summary` lookup now match only notes written by the authenticated user.
- Added a test: a marker planted by another author is ignored by find. Upsert posts a new note instead of PUTting the foreign one, and the publisher still posts its summary.

Fingerprint dedupe in the publisher (`seen`) still accepts any author, as the review scoped this to marker lookups.

---

_Fixed: 2026-11-05_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
