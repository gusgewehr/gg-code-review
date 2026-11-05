# Phase 2 Spike: real GitLab

**Status:** pending user run

## How to run

```
export GITLAB_TOKEN=<personal access token, api scope>
python3 -I docker/gitlab-spike.py --host gitlab.example.com [--ca /path/ca.crt] [--project group/sub/proj] [--with-glab]
```

Run on any machine with python3 that reaches the GitLab host. Output: `gitlab-spike-result.json` (redacted).
The server image has no python3: for `--with-glab`, install glab 1.128.0 on that machine, or run the
same glab commands by hand with `docker compose exec server glab ...` and paste the redacted output below.

## Open questions

| # | Question | Finding | Date |
|---|----------|---------|------|
| 1 | PAT format and length | Finding: | Date: |
| 2 | `/personal_access_tokens/self` availability and fields | Finding: | Date: |
| 3 | GitLab version vs the 16.0 floor | Finding: | Date: |
| 4 | Pagination headers and `glab --paginate` merge semantics | Finding: | Date: |
| 5 | `refs/merge-requests/<iid>/head` via ls-remote | Finding: | Date: |
| 6 | CA handling by glab (`SSL_CERT_FILE` vs `GLAB_CA_CERT`) and git | Finding: | Date: |
| 7 | Exact error strings (TLS, 401, expired, missing scope, unreachable) | Finding: | Date: |
| 8 | `--stdin` trailing newline behavior | Finding: | Date: |
| 9 | Does `glab auth login` validate the token? | Finding: | Date: |
| 10 | `config.yml` layout used by `has_stored_login` | Finding: | Date: |

## What to update after the spike

- The error classifier table (`classify()`) in `backend/src/gitlab/auth.rs` with the real error strings (item 7).
- `has_stored_login` in `backend/src/gitlab/auth.rs` with the real config.yml layout (item 10).
- CA env wiring if `SSL_CERT_FILE` alone is not enough (item 6).
