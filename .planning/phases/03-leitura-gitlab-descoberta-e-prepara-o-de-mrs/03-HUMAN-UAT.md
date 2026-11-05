---
status: partial
phase: 03-leitura-gitlab-descoberta-e-prepara-o-de-mrs
source: [03-VERIFICATION.md]
started: 2026-11-05T00:00:00Z
updated: 2026-11-05T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. glab api output shape against a real GitLab (projects, merge_requests, draft/work_in_progress, 429 wording)
expected: Repository access lists groups/subgroups/projects (Developer+); open MRs map to number=iid, sha, is_draft
result: [pending]

### 2. glab config.yml token layout (hosts:/<host>:/token:, !!null/quoted variants)
expected: git_auth finds the token and git fetch authenticates; without token, fetch fails and preparation fails
result: [pending]

### 3. Fetch refs/merge-requests/<iid>/head and target branch with internal CA, incl. a fork MR
expected: Preparation succeeds with no fork credentials; diff computed locally for a large MR; a failing fetch fails preparation
result: [pending]

### 4. Push to an MR, close/merge an MR, toggle skip_draft_prs on a draft MR
expected: New review supersedes previous; closed/merged MRs leave the queue; drafts follow the option
result: [pending]

### 5. Nested project g/sub/proj: scope, retention and URLs end-to-end
expected: No path or URL breakage in UI, archive and published links
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
