# Milestones

## v1.0 GitLab self-hosted (Shipped: 2026-11-05)

**Delivered:** A deploy set to `OCR_FORGE=gitlab` connects to a self-hosted GitLab with an internal CA, discovers MRs by polling, reviews them in the existing sandbox and publishes inline discussions, status note and summary, with the GitHub flow unchanged.

**Phases completed:** 5 phases, 17 plans
**Git range:** `1d410d8` (docs: map existing codebase) → `v1.0` tag, 122 commits on `docs/codebase-map`
**Audit:** tech_debt, 27/27 requirements, 20/20 integration links ([v1.0-MILESTONE-AUDIT.md](milestones/v1.0-MILESTONE-AUDIT.md))

**Key accomplishments:**

- Forge provider seam (`backend/src/forge/`): all code outside `github/` goes through `forge::`, GitHub URLs and refspec byte-identical
- GitLab connection: `OCR_FORGE` / `OCR_GITLAB_HOST` with a startup pin guard, internal CA bundle, `glab` 1.128.0 pinned with checksum in the server image only, PAT login via stdin in Settings
- GitLab read path: nested groups/projects in Repository access, MR polling with draft/close/merge handling, preparation from `refs/merge-requests/<iid>/head` with live refs and local diff
- GitLab write path: sanitized bodies (no quick actions, mentions or auto-links), inline discussions with general-note fallback, in-place status note, summary, marker idempotency and head-change abort
- Indexing and skills on GitLab, GitLab vocabulary and links in the UI (`forgeTerms`), `docs/gitlab.md` and release note

**Known deferred items at close:** 10 (see STATE.md Deferred Items): real-GitLab UAT in 5 HUMAN-UAT files (25 scenarios) and 5 VERIFICATION files at `human_needed`.

---
