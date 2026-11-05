# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — GitLab self-hosted

**Shipped:** 2026-11-05
**Phases:** 5 | **Plans:** 17 | **Sessions:** autonomous run, 2 days (2026-11-04 → 2026-11-05)

### What Was Built
- Forge provider seam with byte-identical GitHub behavior
- GitLab connection: env-selected forge with pin guard, internal CA, pinned `glab`, PAT login via stdin
- GitLab read path (projects, MR polling, MR head ref preparation) and write path (sanitized inline discussions, status note, summary, idempotency, head abort)
- Indexing, skills, GitLab vocabulary in the UI, `docs/gitlab.md`

### What Worked
- Doing the no-behavior-change seam first kept every later GitLab change behind a single dispatch point
- Fake `glab` scripts let the publisher be tested for fallback, idempotency and abort without an instance
- Code review loops (REVIEW/REVIEW-FIX iterations) caught real issues, such as the broken glab RUN in the Dockerfile closed by 02-05

### What Was Inefficient
- The spike against the real GitLab never ran, so error strings and the `config.yml` layout stayed as `shortcut:` assumptions through phases 3-5
- Phase 1 verification was static only because the toolchain was missing locally; later phases used WSL
- STATE.md was written with Portuguese field labels that `gsd-sdk state.advance-plan` could not parse; REQUIREMENTS.md traceability drifted ("Pendente") until the audit

### Patterns Established
- `forge::` dispatch with a GitHub arm delegating to unchanged `github::` code
- Every GitLab write goes through `gitlab/sanitize.rs`; tokens only travel via stdin or `GIT_CONFIG_*` env
- `shortcut: <limit>, <when to upgrade>` comments for unverified assumptions

### Key Lessons
1. Run the real-instance spike before building on its answers, or keep the dependent code explicitly marked until it runs
2. Keep GSD-parsed field labels (Phase/Plan/Status) in English even when the content is Portuguese
3. Update traceability at each phase close, not at the milestone audit

### Cost Observations
- Model mix: opus orchestration, sonnet for checkers
- Notable: no real GitLab available during the run, so all verifications ended `human_needed`

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | autonomous | 5 | First milestone; real-instance UAT deferred |

### Top Lessons (Verified Across Milestones)

1. (pending a second milestone)
