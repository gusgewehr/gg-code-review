---
status: partial
phase: 04-escrita-gitlab-publica-o-dos-resultados
source: [04-VERIFICATION.md]
started: 2026-11-05T00:00:00Z
updated: 2026-11-05T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Real /merge_requests/:iid/diffs and /versions shapes; inline discussion on added, removed and context lines
expected: Discussions land on the correct line of the MR diff
result: [pending]

### 2. Real status code for an invalid position (expect 400) and the fallback general note
expected: Finding appears as a general note with file and line
result: [pending]

### 3. glab api -X POST -H "Content-Type: application/json" --input on glab 1.128.0 through the internal CA
expected: No 415; body accepted
result: [pending]

### 4. /merge inside a fenced block and a leading /approve in a finding; @user mention
expected: No quick action executes and no one is notified
result: [pending]

### 5. Status note across queued to result
expected: Same note edited in place, no extra notes
result: [pending]

### 6. Push to the MR during publication
expected: Publication aborts and nothing stale is posted
result: [pending]

### 7. Kill the server mid-publication, then retry
expected: No duplicate discussions
result: [pending]

### 8. Token without api scope
expected: Actionable 403 message, no token in logs
result: [pending]

## Summary

total: 8
passed: 0
issues: 0
pending: 8
