---
status: partial
phase: 02-conex-o-com-o-gitlab-self-hosted
source: [02-VERIFICATION.md]
started: 2026-11-05T00:00:00Z
updated: 2026-11-05T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Run docker/gitlab-spike.py against the real GitLab (fill 02-SPIKE.md)
expected: PAT format, personal_access_tokens/self, API version, pagination, CA handling and error strings recorded; classify() and has_stored_login in backend/src/gitlab/auth.rs confirmed or updated
result: [pending]

### 2. Connect with a PAT in Settings > Integrations on a GitLab deploy (OCR_FORGE=gitlab, OCR_GITLAB_HOST, OCR_GITLAB_CA_FILE); try a token without api scope, an expired token, a stopped host; then disconnect
expected: host and user shown; actionable error per case; token absent from browser devtools, server logs and DB; disconnect clears the login
result: [pending]

### 3. Change OCR_FORGE / OCR_GITLAB_HOST over an existing database and restart
expected: server exits with the "pinned to forge ... Restore ..." message
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
