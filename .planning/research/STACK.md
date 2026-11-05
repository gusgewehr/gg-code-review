# Technology Stack: Self-hosted GitLab support

**Project:** OpenCodeReview GitLab fork
**Researched:** 2026-11-05
**Scope:** only what is new for GitLab. The existing Rust/React/Docker/sandbox stack is untouched.

## Recommended Stack

### Hosting CLI (server image only)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `glab` | **1.128.0** (released 2026-11-03; re-check at implementation) | Auth store plus `glab api` transport to the GitLab REST API | Mirrors `gh` (`gh api` -> `glab api`). It is the official GitLab CLI. Supports GitLab 16.0+ officially. |
| `git` (already in image) | bookworm | Fetch of `refs/merge-requests/<iid>/head` and base branch | Same flow as GitHub. Auth injected through env (see below). |

Install in `docker/Dockerfile` (server stage ONLY, never the runner images) with the same pattern as Claude/Codex: `ARG GLAB_VERSION`, `ARG GLAB_SHA256_AMD64/ARM64`, curl the tarball, `sha256sum -c`, extract `bin/glab`, then assert `glab --version`.
- URL: `https://gitlab.com/gitlab-org/cli/-/releases/v${GLAB_VERSION}/downloads/glab_${GLAB_VERSION}_linux_${arch}.tar.gz` (arch = `amd64` | `arm64`; matches `TARGETARCH`).
- Checksums: `https://gitlab.com/gitlab-org/cli/-/releases/v1.128.0/downloads/checksums.txt`. I could not read its contents. Copy the two SHA-256 values at implementation time, do not trust the file at build time. (HIGH that the URLs exist, per the release API.)
- Do NOT use an apt repo or the Debian package for glab: it is not version-pinned and breaks the AGENTS.md "pinned, checksum-verified" rule.
- Add a `provider-cli-canary` entry for glab, like the other CLIs.

### GitLab REST API (v4), all via `glab api`

Always use `glab api --hostname "$OCR_GITLAB_HOST" ...`. Project is addressed by URL-encoded full path (`group%2Fsub%2Fproj`) or numeric id. Prefer storing the numeric project id once discovered, because paths change on rename/transfer.

| Need | Endpoint | Notes |
|------|----------|-------|
| List projects | `GET /projects?membership=true&simple=true&per_page=100&order_by=path` | Use `min_access_level=30` (Developer) so only projects where posting is possible are offered. Needs `simple=true` to keep payload small. |
| List groups | `GET /groups?min_access_level=10&per_page=100` and `GET /groups/:id/subgroups`, `GET /groups/:id/projects?include_subgroups=true` | Workspaces = groups/subgroups. Subgroup `full_path` has several `/` segments, so everything must URL-encode paths. |
| Open MRs (polling) | `GET /projects/:id/merge_requests?state=opened&order_by=updated_at&sort=desc&per_page=100` (optional `updated_after=<ISO8601>`) | The list gives `iid`, `sha` (current diff head), `draft`, `web_url`, `source_project_id`, `author`. Use `sha` for new-head detection. Skip list's `merge_status` (deprecated 15.6). |
| One MR | `GET /projects/:id/merge_requests/:iid` | Has `diff_refs{base_sha,start_sha,head_sha}`. It is filled asynchronously after creation or push, so empty/absent means "retry next poll", not failure. |
| Diff versions | `GET /projects/:id/merge_requests/:iid/versions` | Latest first. Gives `head_commit_sha`, `base_commit_sha`, `start_commit_sha`. The doc says to take position SHAs from here. Fetch it right before posting. (Field names MEDIUM: I confirmed the endpoint is recommended, not the response page.) |
| Inline finding | `POST /projects/:id/merge_requests/:iid/discussions` | `body` plus `position[position_type]=text`, `position[base_sha]`, `position[start_sha]`, `position[head_sha]`, `position[old_path]`, `position[new_path]`, and `position[new_line]` for added lines, `position[old_line]` for removed lines, both for unchanged lines. Returns 201. Optional multi-line via `position[line_range]`; skip in v1. |
| Status / summary comment | `POST /projects/:id/merge_requests/:iid/notes` (create), `PUT .../notes/:note_id` (update), `DELETE .../notes/:note_id` | Plain MR notes, equivalent to GitHub issue comments. |
| Existing notes (idempotency, marker scan) | `GET /projects/:id/merge_requests/:iid/discussions?per_page=100` | Diff notes carry `position`. Scan note `body` for `<!-- ocr-... -->`. Paginate. |
| Reply / resolve | `POST .../discussions/:id/notes`, `PUT .../discussions/:id` with `resolved=true` | Only if the product needs to resolve stale threads. |
| Token owner | `GET /user` | Replaces `gh api user` for the "signed in as" check. |
| Instance version | `GET /version` | Use for a startup warning if below the minimum. |

Send the body via `glab api --method POST ... --input <file>` with a JSON body (same temp-file approach as `github/comments.rs`) rather than `-f`/`-F`. This avoids shell-style quoting issues, and `-F` coerces values that look like integers or `@file`, which would corrupt comment bodies starting with `@` or digits. `--input` sends the body as `application/json` and makes `--field` values query params.

**Diff source: use git, NOT the API.** Compute base/head/merge-base with `git fetch` and `git diff`/`merge-base` on refs, as GitHub does today. `GET /merge_requests/:iid/diffs` is subject to MR diff limits and flags files as `too_large`/`collapsed` (flags added in 18.4), and `/changes` is deprecated since 15.7. The API diff is acceptable only as a cross-check. This also fits the "live base/head/merge-base" invariant.

**Pagination:** `glab api --paginate` is the documented way. `--output ndjson` is documented for large sets (one element per line). Whether default `--output json` merges pages into one array or prints separate arrays is NOT documented. Use `--paginate --output ndjson` and parse line by line, or spike this in phase 1 (LOW on json behavior, MEDIUM on ndjson). Max `per_page` is 100; default 20.

**Minimum GitLab version:** target **16.0+** (glab's official support floor). Required features all predate that (discussions with position, versions, `refs/merge-requests/*`, `diff_refs`, `detailed_merge_status`). `--device` login needs 17.9+, which is why the plan below uses tokens. Use `GET /version` to warn below 16.0. (MEDIUM: floor comes from the glab README, the endpoints are older than that but I did not confirm each one's introduction version.)

### Authentication and token handling

| Item | Decision | Why |
|------|----------|-----|
| Login | `glab auth login --hostname "$OCR_GITLAB_HOST" --stdin --api-protocol https --git-protocol https --insecure-storage` with the PAT on stdin | Non-interactive flags confirmed in docs. Docker has no Secret Service keyring, so the keyring default would fall back or fail. `--insecure-storage` explicitly writes the token to `config.yml`, which is the same threat model as `gh`'s `hosts.yml`. Keep it on a dedicated volume. |
| Config location | `GLAB_CONFIG_DIR=/data/glab` (or similar on the server's volume) | First lookup path, overrides `~/.config/glab-cli`. Mirrors `apply_gh_config_dir`. Never mount into runners. Add `config.yml` to the review archive deny list next to `hosts.yml`. |
| Host | `OCR_GITLAB_HOST` -> pass `--hostname` on every call AND set `GITLAB_HOST` in the child env | Without `--hostname` glab guesses from the git remote, `GITLAB_HOST`, config, then gitlab.com. A wrong guess would send the token to gitlab.com. Always be explicit. |
| Token env | `GITLAB_TOKEN` as a dev fallback only | Takes precedence over stored credentials and applies only to the `GITLAB_HOST` host. Mirrors `GH_TOKEN`. |
| Token for git | `glab auth status --hostname H --show-token --output json` (parse it) and cache like `git_token()` | There is NO `glab auth token` and NO `glab auth git-credential` (confirmed: subcommands are configure-docker, docker-helper, dpop-gen, login, logout, status). Output shape of `--show-token` JSON is unverified, so spike it. Fallback: read `hosts.<host>.token` from `$GLAB_CONFIG_DIR/config.yml`. (LOW until spiked.) |
| Git auth | Same `GIT_CONFIG_COUNT/KEY_0/VALUE_0` env trick as `apply_git_https_auth`: `http.https://<host>/.extraheader = AUTHORIZATION: basic base64("oauth2:<token>")` | GitLab accepts any non-empty username with a PAT as password (documented). Reuse the helper, but the username `x-access-token` currently hard-coded may work too. Use `oauth2` to be conventional. Header via env avoids tokens in argv and `.git/config`. Do not use `glab auth login --setup-git-credential-helper` (it writes global git config, and git prompts hang). |
| Scopes | PAT (or project/group access token) with **`api`** | Posting discussions and notes needs write, so `read_api` is not enough. `read_repository` alone cannot fetch AND post. Token needs Developer role or higher on the project to comment. A bot/project access token is preferable to a person for attribution. |
| Telemetry/updates | `GLAB_SEND_TELEMETRY=false`, `GLAB_CHECK_UPDATE=false`, `NO_PROMPT=1` in the child env | glab defaults check_update=true and telemetry=true. Both are unwanted network calls from a server. NO_PROMPT stops hangs. The `DISABLE_AUTOUPDATER=1` pattern already exists for the AI CLIs. |
| Expiry | Surface "401 Unauthorized" as "GitLab token expired or revoked, run glab login" | PATs default to 365-day expiry, so a silent mid-year failure is likely. |

Sign-in UI must pipe the token to `glab auth login --stdin` and never log it. The project rule is "no app-managed tokens in DB/logs/browser". A one-shot pass-through to glab's own store matches how `gh auth login` is driven today. A device/web OAuth flow (`--web`/`--device`) on self-hosted needs a configured `client_id` and GitLab 17.9+, so it is not a v1 path.

### Git fetch refs

| What | Command | Notes |
|------|---------|-------|
| MR head | `git fetch origin "+refs/merge-requests/<iid>/head:refs/ocr/mr/<iid>/head" --no-tags --quiet` | The existing GitHub flow uses `refs/pull/N/head`. This works for fork MRs too, because GitLab writes the ref into the target project. The docs page for MR refs 404'd, so this is MEDIUM confidence from training plus the standard GitLab behavior. Verify against the real instance in a spike. |
| Target branch | `+refs/heads/<target>:refs/remotes/origin/<target>` (unchanged) | Live base per the invariant. |
| Merge-base | `git merge-base` on fetched refs; cross-check with `diff_refs.base_sha` | `diff_refs.base_sha` is the merge base as GitLab computed it, `start_sha` is the target tip. If they differ from the local result, log it. For inline positions use the API's SHAs (not local ones). |
| Clone URL | `https://<OCR_GITLAB_HOST>/<full_path>.git` | Same shape as `indexing.rs:439`. |
| `refs/merge-requests/*/head` may be absent | GitLab creates it on MR creation and update. Closed MRs keep it. Missing ref means fail the preparation (existing invariant), never fall back. | |

Gotcha: the MR head ref can lag a few seconds after a push. If `versions`/`sha` from the API says head X but the fetched ref is not X, fail and retry on the next poll. Never review a stale head.

### Internal CA trust (debian:bookworm-slim server image)

Recommended: **one mechanism, the system store**, so git, glab (Go), curl all trust it without per-tool config.

1. Compose: mount the PEM(s) read-only, e.g. `./certs/ca.crt:/usr/local/share/ca-certificates/ocr-internal-ca.crt:ro`. The file must end in `.crt`.
2. Server entrypoint (runs as root before dropping privileges, if the image has one; else build-time `COPY` of a `certs/` dir for the Docker-build path): `update-ca-certificates`. `ca-certificates` is already installed (Dockerfile line 21). This regenerates `/etc/ssl/certs/ca-certificates.crt`.
3. Also export `SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt` and `GIT_SSL_CAINFO=/etc/ssl/certs/ca-certificates.crt` in the server env as belt and braces. Go (glab) on Linux honors `SSL_CERT_FILE`/`SSL_CERT_DIR`, and git+libcurl honors `GIT_SSL_CAINFO`.
4. For glab only, if the system-store path is not enough, `glab config set ca_cert /path/ca.pem --host <host>` writes `ca_cert` per host in the config (documented). Per-host config keys confirmed: `ca_cert`, `client_cert`, `client_key`, `skip_tls_verify`.
5. Per-tool git alternative: `GIT_SSL_CAINFO=<file>` env or `http.https://<host>/.sslCAInfo`. Set through the same `GIT_CONFIG_*` env mechanism if the system store is not used. Note `GIT_CONFIG_COUNT` is already used for the extraheader, so keys must be numbered (`KEY_0`, `KEY_1`) and `COUNT` bumped.

Confidence: git and Go honoring system store and `SSL_CERT_FILE` is HIGH from general knowledge, but the glab docs do NOT state it (they document only `ca_cert` and `skip_tls_verify`; there is no documented `GLAB_CA_CERT` env). Smoke-test with a real TLS server signed by a throwaway CA in `docker/` (no credentials needed).

**NEVER use `skip_tls_verify` or `GIT_SSL_NO_VERIFY`** as the default. It would put a bearer token on an unauthenticated channel.

**Runners need nothing:** they never talk to GitLab (egress allowlist is AI hosts only). Do not add the CA or glab to runner images, and do not add the GitLab host to `egress.rs`.

If the CA bundle is delivered through a compose bind mount, document both `OCR_GITLAB_CA_FILE` (host path) and the mount target in `.env.example`. Any rebuild is unnecessary.

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Transport | `glab api` subprocess | Rust HTTP client (`reqwest`/`ureq`) with token | Violates "no app-managed tokens", adds a TLS stack plus CA handling in-process, and diverges from the `gh` architecture. |
| Transport | `glab api` | GitLab Rust crate (`gitlab`) | Same token problem and a big dependency for ~10 endpoints. |
| Diff | git on fetched refs | `/merge_requests/:iid/diffs` | Truncated by instance limits (`too_large`/`collapsed`), paginated, can break the live-ref invariant. |
| Diff | git | `/merge_requests/:iid/changes` | Deprecated since 15.7, scheduled for removal in API v5. |
| Auth store | `--insecure-storage` file under `GLAB_CONFIG_DIR` | OS keyring | No keyring daemon in slim containers. |
| Git auth | `http.<url>.extraheader` via env | `glab auth login --setup-git-credential-helper` | Mutates global git config, needs glab on the git path, risk of prompts. |
| CA | System store plus `SSL_CERT_FILE` | `skip_tls_verify` | Insecure. |
| glab install | pinned tarball plus sha256 | apt repo / `latest` | Not reproducible, breaks AGENTS.md. |
| Sign-in | PAT via stdin | OAuth `--web`/`--device` | Needs a registered `client_id` and 17.9+. Poor fit for a headless server. |

## Installation

```bash
# Dockerfile (server stage), pattern identical to the Codex block
ARG GLAB_VERSION=1.128.0
ARG GLAB_SHA256_AMD64=<from checksums.txt>
ARG GLAB_SHA256_ARM64=<from checksums.txt>
# curl -fsSL --retry 3 ".../v${GLAB_VERSION}/downloads/glab_${GLAB_VERSION}_linux_${arch}.tar.gz" -o /tmp/glab.tgz
# sha256sum -c ; tar -xzf ; install -m0755 bin/glab /usr/local/bin/glab ; glab --version check

# Env for the server (compose)
# OCR_GITLAB_HOST=gitlab.corp.example   GLAB_CONFIG_DIR=/data/glab
# GLAB_SEND_TELEMETRY=false GLAB_CHECK_UPDATE=false NO_PROMPT=1
# SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt GIT_SSL_CAINFO=/etc/ssl/certs/ca-certificates.crt
```

No new Rust or npm dependencies are needed (JSON via the existing `serde_json`).

## Sources

- glab releases (GitLab API, 2026-11-03 v1.128.0, asset names): https://gitlab.com/api/v4/projects/gitlab-org%2Fcli/releases - HIGH
- glab `auth login` flags/env/storage: https://docs.gitlab.com/cli/auth/login/ - HIGH
- glab `auth status`, no `auth token`: https://docs.gitlab.com/cli/auth/status/ - HIGH
- glab `api` flags: https://docs.gitlab.com/cli/api/ - HIGH (pagination merge semantics undocumented)
- glab config/env/ca_cert: https://docs.gitlab.com/cli/configuration/ and https://docs.gitlab.com/cli/connection/ - HIGH
- glab supported GitLab versions (16.0+): https://gitlab.com/gitlab-org/cli/-/raw/main/README.md - HIGH
- Merge requests API (list, diff_refs, diffs, changes deprecation): https://docs.gitlab.com/api/merge_requests/ - HIGH
- Discussions API (position params, versions): https://docs.gitlab.com/api/discussions/ - HIGH
- REST pagination: https://docs.gitlab.com/api/rest/ - HIGH
- PAT git-over-HTTPS username: https://docs.gitlab.com/user/profile/personal_access_tokens/ - HIGH
- Existing code patterns: `backend/src/github/auth.rs` (extraheader), `transport.rs`, `docker/Dockerfile` - HIGH
- `refs/merge-requests/<iid>/head` behavior, SSL_CERT_FILE/GIT_SSL_CAINFO honoring, project/group list params - MEDIUM/LOW (training knowledge, not verified against fetched docs)
