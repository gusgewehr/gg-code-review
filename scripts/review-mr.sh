#!/bin/sh
# Reviews one GitLab merge request and writes reviews/mr-<iid>/review.md. Nothing is posted to
# GitLab. See docs/mr-review.md.
#
#   export GITLAB_TOKEN=...              # personal access token, scopes read_api + read_repository
#                                        # (or GITLAB_PASSWORD holding that token, glpat-...)
#   export CLAUDE_CODE_OAUTH_TOKEN=...   # from `claude setup-token`
#   export OCR_GITLAB_CA_FILE=~/ca.pem   # optional: internal CA (PEM)
#   scripts/review-mr.sh https://gitlab.example.com/group/project/-/merge_requests/42
#
# Tokens are passed to Docker by name, never as values on a command line.
set -eu

url=${1:?usage: scripts/review-mr.sh <merge request URL>}
# A personal access token works with any user name, so GITLAB_USER is not needed.
GITLAB_TOKEN=${GITLAB_TOKEN:-${GITLAB_PASSWORD:-}}
: "${GITLAB_TOKEN:?export GITLAB_TOKEN or GITLAB_PASSWORD (a personal access token with scopes read_api, read_repository) first}"
export GITLAB_TOKEN
image=${OCR_MR_IMAGE:-opencodereview-mr:local}
root=$(cd "$(dirname "$0")/.." && pwd)

# Rebuild when the checkout moved since the image was built (a `git pull`), so it never runs stale code.
commit=$(git -C "$root" rev-parse HEAD 2>/dev/null || echo unknown)
built=$(docker image inspect -f '{{index .Config.Labels "ocr.commit"}}' "$image" 2>/dev/null || true)
if [ "${OCR_MR_REBUILD:-}" = 1 ] || [ "$built" != "$commit" ]; then
  docker build -f "$root/docker/Dockerfile" -t opencodereview-server:mr "$root"
  docker build -f "$root/docker/Dockerfile.bench" --build-arg OCR_RUNTIME_IMAGE=opencodereview-server:mr \
    --label "ocr.commit=$commit" -t "$image" "$root"
fi

set -- docker run --rm -e GITLAB_TOKEN -v "$(pwd)/reviews:/work/out"
if [ -n "${OCR_BENCH_CREDENTIAL_FILE:-}" ]; then
  set -- "$@" -v "$(realpath "$OCR_BENCH_CREDENTIAL_FILE"):/run/ocr-bench/credential:ro"
else
  : "${CLAUDE_CODE_OAUTH_TOKEN:?export CLAUDE_CODE_OAUTH_TOKEN (claude setup-token) or OCR_BENCH_CREDENTIAL_FILE}"
  set -- "$@" -e CLAUDE_CODE_OAUTH_TOKEN
fi
if [ -n "${OCR_GITLAB_CA_FILE:-}" ]; then
  set -- "$@" -v "$(realpath "$OCR_GITLAB_CA_FILE"):/run/ocr-ca/ca.crt:ro"
fi
for name in RB_CONFIG_PROVIDER RB_CONFIG_MODEL RB_CONFIG_EFFORT RB_CONFIG_MAX_FILES RB_CONFIG_FOCUS_FILES \
  RB_CONFIG_SUGGESTIONS RB_CONFIG_DEADLINE_SECS; do
  if [ -n "$(printenv "$name" || true)" ]; then
    set -- "$@" -e "$name"
  fi
done
mkdir -p reviews
exec "$@" "$image" mr "$url"
