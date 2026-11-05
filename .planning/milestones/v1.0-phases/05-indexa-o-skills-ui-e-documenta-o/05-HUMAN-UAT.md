---
status: partial
phase: 05-indexa-o-skills-ui-e-documenta-o
source: [05-VERIFICATION.md]
started: 2026-11-05T00:00:00Z
updated: 2026-11-05T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Index a real GitLab project (nested subgroup) with OCR_AUTO_REINDEX=true
expected: Clone succeeds through the internal CA; metadata, package graph and symbols appear as on GitHub
result: [pending]

### 2. Skills on a GitLab project review
expected: A selected skill version is applied to a GitLab MR review the same way as on GitHub
result: [pending]

### 3. GitLab deploy UI vocabulary and links
expected: Reviews, detail, run details, usage and settings say merge request / MR and `!iid`; "Open on GitLab" opens the MR on the instance; no GitHub mark
result: [pending]

### 4. GitHub deploy UI unchanged
expected: Same text, `#n`, GitHub mark and links as before Phase 5
result: [pending]

### 5. docs/gitlab.md walkthrough, including the docker/gitlab-spike.py command
expected: A fresh deploy configured only from the doc connects and reviews an MR
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0

## Gaps
