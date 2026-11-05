---
phase: 05-indexa-o-skills-ui-e-documenta-o
plan: 01
requirements-completed: [IDX-01, IDX-02, UI-01]
---
# Phase 5 Plan 01: GitLab indexing gate, clone_url, sign-in copy

Removed the GitHub-only reindex gate in server/mod.rs, extracted a tested `clone_url(host, full_name)` helper (nested GitLab paths work), and made the generic GitLab sign-in error cite glab and the PAT (scope api).

## IDX-02 proof
- `grep -nE "forge|github|split_once|rsplit_once" backend/src/db/skills.rs`: no output
- `grep -n "github\.com" src/platform/lib/skillEditor.ts`: no output
- `grep -B8 active_skill_content backend/src/server/review_jobs.rs | grep -c "forge::"`: 0

## Verification
fmt and clippy -D warnings clean; `--lib clone_url` passes. `gitlab::auth` and `indexer` filters and the file-size check were not run.

## Deviations
None.
