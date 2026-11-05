---
status: partial
phase: 01-camada-de-provider-forge-sem-mudan-a-de-comportamento
source: [01-VERIFICATION.md]
started: 2026-11-05T00:00:00Z
updated: 2026-11-05T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. cargo fmt --all --check; cargo clippy --workspace --all-targets --locked -- -D warnings; cargo test --workspace --locked (with OCR_TEST_DATABASE_URL)
expected: All pass; forge::tests::github_urls_and_refspec_are_unchanged passes; no existing assertion edited
result: [pending]

### 2. pnpm test
expected: Passes (no frontend change in this phase)
result: [pending]

### 3. Rebuild server+runner images together; run docker/runner-smoke.py and the other Docker smokes
expected: Pass unchanged (executor fingerprint changes, so both images must be rebuilt together)
result: [pending]

### 4. Review and publish a real GitHub PR; compare comment bodies and URLs with a pre-refactor run
expected: Byte-identical bodies and URLs
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
