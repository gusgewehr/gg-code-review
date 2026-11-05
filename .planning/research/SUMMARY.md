# Research Summary — Self-hosted GitLab support

**Date:** 2026-11-05 · **Overall confidence:** MEDIUM
**Sources:** STACK.md (MEDIUM-HIGH, official docs), FEATURES.md (MEDIUM), ARCHITECTURE.md (HIGH on code, MEDIUM/LOW on GitLab), PITFALLS.md (MEDIUM)

## Executive Summary

Brownfield change. OpenCodeReview already polls, reviews in a sandbox, validates findings against a frozen archive and publishes idempotently through `gh`. The work is a provider seam in front of the ~26 files that call `github::` directly:

- **Seam:** a `Forge` trait chosen once per deploy (`OCR_FORGE`, `OCR_GITLAB_HOST`). `backend/src/github/` is wrapped, not rewritten; new `backend/src/gitlab/` drives `glab api`.
- **Unchanged:** runners, egress allowlist, DB schema, review invariants. No new Rust/npm dependencies, no DB migration for v1.
- **Where GitLab runs:** only in `server`. `glab` 1.128.0 pinned and checksum-verified in the server image.
- **Diffs:** `git fetch refs/merge-requests/<iid>/head` + local `git diff` (REST diffs are truncated; `/changes` deprecated since 15.7).
- **Inline positions:** `base_sha`/`start_sha`/`head_sha` from the latest `/versions` entry fetched right before posting, never from local git.
- **Internal CA:** installed in the server system store, plus `SSL_CERT_FILE` and `GIT_SSL_CAINFO`. No skip-verify.

**Main risks:** (1) GitHub regression during the refactor; (2) inline-position correctness; (3) no atomic review object on GitLab; (4) unverified `glab` behaviors — needs an early spike against the real instance.

## Key Findings

### Stack
- `glab` 1.128.0 from a pinned tarball with sha256, plus a provider-cli canary entry. Supports GitLab ≥ 16.0.
- POST bodies via `glab api --input <file>`, never `-f`/`-F`. Child env: `GLAB_SEND_TELEMETRY=false`, `GLAB_CHECK_UPDATE=false`, `NO_PROMPT=1`.
- Auth: PAT with `api` scope via `glab auth login --hostname H --stdin --insecure-storage`, `GLAB_CONFIG_DIR` on the server volume, `config.yml` on the archive deny list.
- No `glab auth token`: git token from `glab auth status --show-token --output json` or `config.yml` (unverified). Git auth reuses the `GIT_CONFIG_*` extraheader with `oauth2:<token>`.

### Features
- **Table stakes:** inline resolvable discussions; fallback to general note with same marker; marker-based idempotency; one status note edited in place + summary note; supersede on new head; iid-keyed identity; nested groups and forks; draft policy mirroring GitHub; pagination and 429 handling; never approve/merge/push.
- **Defer:** inline suggestion blocks (contradicts "suggestions live in the summary" invariant), auto-resolve, labels, commit statuses.
- **Anti-features:** webhooks, slash commands/chat, approvals, managed tokens, MR title/description rewrites, runner access to GitLab.

### Architecture
- `forge/` holds trait, neutral types, diff parser, dedupe, transport. GitLab impl returns GitHub-shaped states/payloads/BIGINT ids so `publication.rs` doesn't branch on provider.
- Provider guard: `forge_provider`/`forge_host` in `settings`; boot with a different value refuses to start.
- Fixed during refactor: two-segment `full_name` rule (`Scope::validate`, retention `split_once('/')`), hard-coded `github.com` URLs, `refs/pull` refspec (`review/git.rs`, `server/summary.rs`), timestamp parser ignoring offsets, string-matched error classification, `x-access-token` git username.
- Review identity on GitLab: post inline discussions first, then the top-level marker note last; its note id is the "review id". Fingerprint dedupe makes retry after partial failure safe.
- File-size pressure: `publication.rs`, `chunks.rs`, `ReviewsPage.tsx`, `reviewFlow.ts` near caps — new code goes in new modules.

### Pitfalls (top 5)
1. Wrong position SHAs → fetch `/versions` right before posting; assert response `type == "DiffNote"`.
2. `old_line`/`new_line` rules → added: `new_line` only; removed: `old_line` only; context: both; `old_path` and `new_path` always.
3. Outdated positions after push → re-read MR head before posting, abort on mismatch.
4. TLS across three trust stores (glab/Go, git, Rust) → one mounted bundle, server only.
5. Body hazards → lines starting with `/` run quick actions; `@`, `#`, `!`, ```` ```suggestion ```` have side effects. Neutralize in a GitLab post-processor; read markers from `body`, not `body_html`.

## Roadmap Implications

1. **Forge seam, GitHub only, zero behavior change** — trait/types, `impl Forge for GitHub`, call sites moved, neutral URLs/refspec, `FakeForge`. Gate: existing tests + runner-smoke green, GitHub bodies/URLs byte-identical, zero `github::` refs outside `backend/src/github/`.
2. **Spike + provider selection, image, CA, auth** — spike on real instance, `OCR_FORGE` guard, pinned `glab`, CA install, PAT sign-in with scope/expiry preflight, `glpat-` redaction test. Gate: throwaway-CA TLS smoke for `glab api` and `git fetch`.
3. **GitLab read path** — groups/subgroups/projects listing, open-MR polling, head detection, MR-ref fetch verified against API head, local diff, pagination/backoff. Gate: real MR (incl. fork MR and 3-level subgroup) reviewed with fake inference, nothing posted.
4. **GitLab write path** — inline discussions, note fallback, status + summary notes, marker-last identity, stale-position handling, body sanitizer/truncation. Gate: PITFALLS smoke checklist on real instance incl. partial failure then retry.
5. **Indexing, skills, UI** — forge terms (MR, `!iid`, `web_url`), forge field in status payload, Docker sign-in guidance, docs, changelog.
6. **Optional hardening** — multi-line `line_range`, inline suggestions, auto-resolve, labels, upstream PR.

### Research flags
- Needs spike/research: Phase 2 (verification spike, CA smoke), Phase 3 (`--paginate` shape, MR-ref lag, `base_sha` source), Phase 4 (`/versions` fields, position errors, marker survival, quick-action inertness in fences).
- Standard patterns: Phase 1 (codebase is the spec), Phase 5.

## Resolved Conflicts

- Diff source: local git (STACK, PITFALLS) over `glab mr diff --raw` (ARCHITECTURE).
- Git token: STACK path, confirmed by spike.
- Minimum role: STACK `min_access_level=30` filter plus preflight on real access level.
- Position SHAs: latest `/versions` entry, cross-checked with `diff_refs.head_sha`.
- `--paginate`: `--output ndjson` parsed per line unless spike says otherwise.
- Sign-in UI: v1 shows status + `docker compose exec server glab auth login ...` guidance; no in-UI PAT form.
- `LivePrState.base_sha`: live target-branch tip; API SHAs only for positions.

## Open Questions (for the spike)

- `--show-token` JSON shape; `--paginate` shape; glab honoring the internal CA full chain; `/versions` fields.
- Fork-MR ref presence and lag; marker survival and `/merge` inertness in a note; target GitLab version; relative URL root.
- `line_range` and renamed-file positions; collapsed/too-large diff behavior.
- Draft policy: mirror `polling_pr.rs`.
- Where `review/dependencies.rs` is entered (for `supports_external_dependencies()`).
- Whether a bot user holds the PAT, and its expiry.
