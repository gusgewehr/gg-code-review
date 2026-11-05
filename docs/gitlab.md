# Running OpenCodeReview against a self-hosted GitLab

One deploy talks to one forge. A GitLab deploy discovers open merge requests by polling, reviews them
in the existing sandbox and publishes the findings as inline discussions on the merge request.

## Requirements

- Docker Compose.
- A self-hosted GitLab reachable from the server container.
- A dedicated bot user with Developer access or higher on the projects to review. Recommended, so
  findings are posted as that user.

## Configure

| Variable | Value |
|----------|-------|
| `OCR_FORGE` | `gitlab` (default is `github`) |
| `OCR_GITLAB_HOST` | Bare host such as `gitlab.example.com`, no scheme and no path |
| `OCR_GITLAB_CA_FILE` | Host path of the internal CA certificate (PEM file). Mounted read-only at `/run/ocr-ca/ca.crt` for the server only; runners never get it |

```sh
OCR_FORGE=gitlab \
OCR_GITLAB_HOST=gitlab.example.com \
OCR_GITLAB_CA_FILE=/etc/ssl/internal-ca.pem \
docker compose up --build
```

The forge and host are pinned on first start. Changing them over existing data stops the server.
Mount your internal CA instead of disabling TLS verification; there is no insecure option.

## Create the access token

Create a personal access token for the bot user with scope `api` and set an expiry. Paste it in
Settings > Integrations > GitLab connection. The token is handed to `glab` on stdin and is never stored
by OpenCodeReview. Do not put it in `.env` files, compose files or the shell history of a shared host.

## Pick projects

Open Settings > Repository access and select groups, subgroups or projects. Open merge requests are
discovered by polling every `OCR_POLL_INTERVAL_SECONDS`.

## Check connectivity first

Export `GITLAB_TOKEN` in your shell (environment only, never on the command line), then run:

```sh
python3 -I docker/gitlab-spike.py --host gitlab.example.com --ca ca.crt --with-glab --project group/sub/proj
```

The result is written, redacted, to `gitlab-spike-result.json`.

## Troubleshooting

- TLS errors or host unreachable: check the CA mount. `OCR_GITLAB_CA_FILE` must be a PEM file, not a
  directory.
- 401, expired token or missing scope: create a new token with scope `api`.
- Sign-in failed: the connection uses `glab` with the personal access token; reconnect in Settings.

## Limitations (v1)

- One forge per deploy.
- Polling only, no webhooks.
- OpenCodeReview never approves, requests changes on or merges a merge request. Request changes
  auto-post publishes comments.
- Pinned dependencies from submodules or linked repositories are not captured on GitLab; the review
  uses the checkout's own evidence.
- `glab` ships only in the server image.
