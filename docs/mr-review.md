# Reviewing a GitLab merge request on demand

`scripts/review-mr.sh <merge request URL>` reviews one merge request with the OpenCodeReview
pipeline and writes `reviews/mr-<iid>/review.md`: a summary, then every finding, suggestion and
question with its `path:line`, title, explanation and suggested fix, ready to copy into the merge
request by hand. **Nothing is posted to GitLab**, and the token only needs read scopes.

One container does the whole run and exits: a throwaway Postgres, one sandboxed review runner and
`ocr-bench mr`, which fetches the merge request and plays the server's part. The model only reads
the frozen snapshot of the diff, through the runner, with egress limited to the AI provider.

## Requirements

- Docker (on Windows, run the script from WSL).
- A GitLab personal access token with scopes `read_api` (merge request metadata) and
  `read_repository` (git fetch). Your own token is enough: the reviewer reads what you can read.
  Put it in `GITLAB_TOKEN`, or in `GITLAB_PASSWORD` if your environment already keeps it there
  (`GITLAB_TOKEN` wins when both are set; `GITLAB_USER` is not needed).
- A Claude sign-in token from `claude setup-token`, or for Codex a sign-in file (below).
- If the GitLab instance uses an internal CA: its certificate as a PEM file.

## Run

```sh
export GITLAB_TOKEN=glpat-...            # read_api + read_repository
export CLAUDE_CODE_OAUTH_TOKEN=sk-ant-... # claude setup-token
export OCR_GITLAB_CA_FILE=~/corp-ca.pem   # optional
scripts/review-mr.sh https://gitlab.example.com/group/sub/project/-/merge_requests/42
```

The first run builds the images (`opencodereview-server:mr`, then `opencodereview-mr:local`), and
so does the first run after a `git pull` that moved the checkout; `OCR_MR_REBUILD=1` forces it. The report is printed and saved under
`./reviews/mr-42/` together with `ocr-findings.json` (full JSON) and `findings.json`.

Export tokens in your shell session (or read them from a password manager); don't put them in
files in the repository or on the command line.

## Options

| Variable | Default | Effect |
|----------|---------|--------|
| `RB_CONFIG_PROVIDER` | `claude` | `codex` to review with Codex; then set `OCR_BENCH_CREDENTIAL_FILE` to its `auth.json` |
| `RB_CONFIG_MODEL`, `RB_CONFIG_EFFORT` | product defaults | Model and reasoning effort |
| `RB_CONFIG_LANGUAGE` | `Brazilian Portuguese (pt-BR)` | Language of titles, explanations and fixes (code and quotes stay as they are), e.g. `English` |
| `RB_CONFIG_SUGGESTIONS` | `true` | Lower-priority suggestions (tests, naming, duplication) besides findings |
| `RB_CONFIG_MAX_FILES` | `800` | Larger merge requests are declined |
| `RB_CONFIG_FOCUS_FILES` | `100` | Above this, the highest-priority files are reviewed and the rest listed |
| `RB_CONFIG_DEADLINE_SECS` | `3600` | The review stops here and reports what finished chunks found |

## Troubleshooting

- `Could not reach <host>`: network or TLS. Set `OCR_GITLAB_CA_FILE` to the internal CA PEM.
- `GitLab rejected GITLAB_TOKEN`: the token expired or lacks `read_api`.
- `Could not fetch <project>`: the token lacks `read_repository`.
- `was not found, or the token cannot read it`: wrong URL or no access to the project.
- `Status: failed` in the report: the review's own error is quoted under it.
