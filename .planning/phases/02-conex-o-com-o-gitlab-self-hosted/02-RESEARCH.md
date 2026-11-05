# Phase 2: Conexão com o GitLab self-hosted - Research

**Researched:** 2026-11-05
**Domain:** Rust server config guard, CLI (`glab`) credential pass-through, internal CA trust, Settings UI panel
**Confidence:** MEDIUM-HIGH (code and glab docs verified; glab runtime behavior/error texts need the spike)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Env `OCR_FORGE=github|gitlab` (default `github`) e `OCR_GITLAB_HOST` (host sem esquema, ex. `gitlab.empresa.local`; obrigatório quando `OCR_FORGE=gitlab`).
- Guarda: na primeira subida grava `forge_provider` e `forge_host` na tabela de settings existente (sem migration nova se settings for chave/valor; se precisar de coluna, migration nova no fim da lista). Subida posterior com valor diferente → server sai com mensagem clara dizendo qual valor mudou e como reverter.
- Deploy GitHub existente sem as chaves gravadas: grava `github` + host atual na primeira subida, sem quebrar.
- `Forge` ganha a variante `GitLab` (enum da Fase 1); operações ainda não implementadas retornam erro claro "not supported yet on GitLab" em vez de panic.
- CA: bundle opcional montado via compose em caminho fixo (ex. `/run/ocr-ca/ca.crt`, volume read-only só no serviço `server`). No boot, o server concatena o bundle do sistema + CA interna num arquivo em `OCR_DATA_DIR` e exporta `SSL_CERT_FILE` e `GIT_SSL_CAINFO` para os processos filhos (`glab`, `git`). Nenhuma flag de skip-verify existe. Runners não montam a CA nem recebem `glab` ou token (AUTH-05).
- `glab` 1.128.0 baixado no `docker/Dockerfile` (server) de tarball oficial com sha256 fixo, igual ao padrão de Claude/Codex CLIs. Não entra em `Dockerfile.runner`. Env dos filhos: `GLAB_CONFIG_DIR` no volume de dados do server, `GLAB_SEND_TELEMETRY=false`, `GLAB_CHECK_UPDATE=false`, `NO_PROMPT=1`.
- Settings ganha um painel GitLab (espelho do bloco GitHub, reusando `SettingsControls`) com campo password para o PAT, botão Connect e Disconnect. Só quando o deploy é GitLab; no deploy GitHub a UI não muda.
- Endpoint `POST` recebe o token no corpo, repassa via stdin para `glab auth login --hostname H --stdin` e nunca o loga, ecoa ou grava no banco. Respostas de erro não contêm o token.
- Status: `glab api user` → username; `glab api personal_access_tokens/self` → escopos e `expires_at`. Erros acionáveis: sem escopo `api`, expirado, 401, host fora do ar/erro TLS (mensagem cita a CA).
- Disconnect: `glab auth logout --hostname H`.
- Teste de redação: string `glpat-...` nunca aparece em logs/respostas.
- Spike: sem acesso à instância real. Entregar `docker/gitlab-spike.py` + `SPIKE.md` que o usuário roda depois. Não bloqueia a fase.

### Claude's Discretion
Nomes de endpoints, layout exato do painel, estrutura de módulos em `backend/src/gitlab/`. Velocidade acima de polimento.

### Deferred Ideas (OUT OF SCOPE)
- Confirmação real do spike (formato `--show-token`, paginação, `/versions`) — depende do usuário rodar o script.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FORGE-01 | Provider + host por config; server recusa subir se mudarem sobre dados existentes | Guard pattern below (`ensure_setting_default` + compare), `forge::init` OnceLock |
| AUTH-01 | Admin cola PAT em Settings, vê host+usuário | `POST /auth/gitlab/token`, `GET /forge/status`, `GitLabConnection.tsx` |
| AUTH-02 | Erros acionáveis (escopo `api`, expirado, host fora, TLS) | Status classification section |
| AUTH-03 | Disconnect | `glab auth logout --hostname` |
| AUTH-04 | Server confia na CA interna, sem skip-verify | CA bundle builder, `SSL_CERT_FILE`/`GIT_SSL_CAINFO`/`GLAB_CA_CERT` |
| AUTH-05 | Token nunca em browser/log/DB; runner sem glab/token/CA | Redaction, Dockerfile server-only, static check on runner files |
</phase_requirements>

## Summary

The GitHub path is a good template: `github/auth.rs` already shows the config-dir, host validation, `Command` env and sign-out patterns, and the settings table is plain key/value (`settings(key,value,updated_at)`), so **no migration is needed**. `save_setting` (the HTTP-writable path) is whitelisted by `ALLOWED_SETTING_KEYS`, so `forge_provider`/`forge_host` can NOT be tampered with from the browser as long as they are not added to that list. Use `ensure_setting_default` (`INSERT ... ON CONFLICT DO NOTHING`, currently `pub(super)` in `db/settings.rs`; widen to `pub(crate)` or add a `Db` method) then read back with `setting()`; this is atomic across multi-instance startup.

The frontend does not know the forge today: `setupApi.setupStatus()` is built client-side from `/auth/github/status`, and `SettingsPage.tsx` shows a `GitHubAuthRecovery` banner and a GitHub `IntegrationRow` whenever that is disconnected. On a GitLab deploy those would be wrong, so add a tiny `GET /forge/status` returning `{provider, host, loggedIn, login, expiresAt}` and make SettingsPage branch on it.

Two research corrections versus STACK.md: (1) glab **does** document `GLAB_CA_CERT` (alias `CA_CERT`, config key `ca_cert`) [CITED: docs.gitlab.com/cli/configuration]; set it in addition to `SSL_CERT_FILE` as belt and braces. (2) `checksums.txt` was readable; sha256 values are below.

**Primary recommendation:** New `backend/src/gitlab/{mod,auth,ca}.rs` + `backend/src/forge/guard.rs`; process-wide `set_var` for `SSL_CERT_FILE`/`GIT_SSL_CAINFO` at boot before any thread starts; PAT goes server memory → glab stdin only; status derived from glab (no DB state); spike delivered as script + SPIKE.md, non-blocking.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Provider/host selection and guard | API / Backend (boot) | Database (settings) | Deploy config in env, pinned in `settings` |
| CA bundle build + env export | API / Backend (boot) | Compose mount | Server children (`glab`, `git`) only; runners excluded |
| PAT login/logout/status | API / Backend (spawns glab) | Browser (form only) | Token must never persist in browser/DB; glab owns the store |
| Connection panel | Browser / Client | — | Renders status; clears token state on submit |
| Knowing the deploy forge | API (`/forge/status`) | Browser | Forge is server config, not a user setting |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| glab | 1.128.0 | PAT store + `glab api` transport | Locked; official GitLab CLI, mirrors `gh` |
| existing std/serde_json/`std::process::Command` | — | glab spawn, JSON parse | No new crates; no new npm deps |

**No new Rust crates and no npm packages are installed** [VERIFIED: codebase]. Package Legitimacy Audit: not applicable to registries; the only external artifact is the glab tarball, verified by pinned sha256 (below). slopcheck not needed.

### glab 1.128.0 download + checksums
[VERIFIED: fetched `https://gitlab.com/gitlab-org/cli/-/releases/v1.128.0/downloads/checksums.txt` on 2026-11-05]
```
bc024819e7598b19429a199cd87f640d98b2b67416d8ad796f19e4260b57a7c7  glab_1.128.0_linux_amd64.tar.gz
60d0512085a8c1be7c5ef51aa7e4ee28213444cea0376b3260707d90489199ea  glab_1.128.0_linux_arm64.tar.gz
```
URL: `https://gitlab.com/gitlab-org/cli/-/releases/v${GLAB_VERSION}/downloads/glab_${GLAB_VERSION}_linux_${arch}.tar.gz` (arch = `$TARGETARCH`: amd64|arm64; the Dockerfile already builds both). Tarball layout (`bin/glab`) is [ASSUMED]; plan task should `tar -tzf` once and the Dockerfile must end with `glab --version` assertion. Insert the block right after the Codex block in `docker/Dockerfile` (runtime stage), same shape: `case "$TARGETARCH"`, `curl -fsSL --retry 3`, `printf '%s  file\n' "$sum" | sha256sum -c -`, `tar`, `install -m0755`, `LABEL ai.glab.cli.version`. Do NOT touch `docker/Dockerfile.runner` (confirmed it has no gh/glab today). Also add glab to `docker/provider-cli-canary.py` if it enumerates CLIs (STACK.md suggestion, optional).

## Architecture Patterns

### Flow
```
compose env OCR_FORGE/OCR_GITLAB_HOST + ca.crt mount
   -> ServerConfig::from_env / run(): validate host -> forge::guard(db) (ensure default, compare, Err(msg) => process exits)
   -> gitlab::ca::prepare(): concat system bundle + ca.crt -> $OCR_DATA_DIR/ca-bundle.crt (0600)
        -> set_var SSL_CERT_FILE, GIT_SSL_CAINFO, GLAB_CA_CERT (before threads)
   -> forge::init(Forge::GitLab{host}) OnceLock
Browser: PAT in form --POST /auth/gitlab/token {token}--> routes.rs (origin+json checks)
   -> gitlab::auth::login: glab auth login --hostname H --stdin --api-protocol https --git-protocol https --insecure-storage (token on stdin)
   -> verify: glab api user ; glab api personal_access_tokens/self ; on failure => glab auth logout + actionable error
Browser: GET /forge/status -> {provider, host, loggedIn, login, expiresAt} (derived from glab, cached ~30-60s, invalidated on login/logout)
```

### Recommended structure
```
backend/src/forge/mod.rs      # Forge::{GitHub, GitLab}, init()/current() via OnceLock (default GitHub for tests)
backend/src/forge/guard.rs    # pure fn check(stored, configured) -> Result<(), String>; db wrapper
backend/src/gitlab/mod.rs     # host validation (reuse github::is_valid_gh_host logic), apply_glab_env(cmd)
backend/src/gitlab/auth.rs    # login/status/logout, classify(), redact()
backend/src/gitlab/ca.rs      # build_bundle(system, extra, out) -> Result
src/platform/pages/settings/GitLabConnection.tsx
```

### Pattern 1: Guard (FORGE-01)
- Configured values from env: provider (`github` default; anything else than github|gitlab => Err), host: gitlab => required + validated (bare `host[:port]`, reuse `is_valid_gh_host` rules, lowercase); github => `OCR_GH_HOST` if valid else `github.com` (**do not call `gh_host()`**: it shells out to `gh auth status` and can vary between boots, which would make the guard flap). This is a small deviation from "host atual": deterministic env-only host.
- After migrations, in `run()` right after `config.db.open()`: `db.ensure_setting_default("forge_provider", p)`, same for host, read back with `db.setting(..)`, compare. Mismatch => `Err(format!("Deploy is pinned to {stored_p}@{stored_h} but OCR_FORGE/OCR_GITLAB_HOST say {p}@{h}. Revert the env (or use a fresh database); changing forge or host over existing data is not supported."))`. `run_from_env` returns `Err`; confirm `main_server.rs` prints it and exits non-zero (planner: read it, 10 lines).
- Keep the pure compare function unit-testable without a DB; add one Postgres test for first-boot-writes and mismatch (uses `OCR_TEST_DATABASE_URL`).
- Never add the two keys to `ALLOWED_SETTING_KEYS` (keeps them read-only from the API) [VERIFIED: db/settings.rs:157-193].

### Pattern 2: Forge enum
Add `GitLab`; `current()` reads `OnceLock<Forge>` (set once in boot; tests keep GitHub default). Existing three methods: implement GitLab formatting (cheap, pure): `pr_head_ref` => `refs/merge-requests/{n}/head`; `commit_url`/`review_url` need host, so keep host in a second `OnceLock<String>` (`forge::host()`); `review_url` => `https://{host}/{full}/-/merge_requests/{n}` (Phase 4 refines). Anything else needing GitLab returns `Err("not supported yet on GitLab")`. Because `forge/mod.rs` does `pub(crate) use crate::github::*`, GitLab auth functions must live in `crate::gitlab` and be called explicitly (no name collision with `forge::auth_status`).
Gate GitHub-only background work on GitLab deploys so logs are not spammed by failing `gh` calls: skip/early-return `start_poll_scheduler`, `start_reindex_worker`, `github_status()` probe in `/health` when `forge::current() != GitHub` (Phase 3 replaces them). Planner: locate each in `server/mod.rs` `run()`.

### Pattern 3: glab child env helper
```rust
// gitlab/mod.rs  (mirrors apply_gh_config_dir/apply_gh_host)
pub(crate) fn apply_glab_env(cmd: &mut Command, host: &str) {
    cmd.env("GLAB_CONFIG_DIR", config_dir())      // $OCR_DATA_DIR/glab, create_private_dir (0700)
       .env("GITLAB_HOST", host)                  // never let glab guess gitlab.com
       .env("GLAB_SEND_TELEMETRY", "false")
       .env("GLAB_CHECK_UPDATE", "false")
       .env("NO_PROMPT", "1")
       .env_remove("GITLAB_TOKEN").env_remove("GITLAB_ACCESS_TOKEN").env_remove("OAUTH_TOKEN");
}
```
Always also pass `--hostname H` explicitly. `create_private_dir` is `pub(crate)` in `github/auth.rs`; reuse (re-exported via forge glob or call `crate::github::auth::create_private_dir`; check visibility of `auth` module).

### Pattern 4: Login (AUTH-01, AUTH-05)
```rust
// body type: custom so serde errors cannot echo the token
#[derive(Deserialize)] struct TokenBody { token: String }
let body: TokenBody = serde_json::from_slice(&req.body).map_err(|_| "Invalid request body.".to_string())?;
let token = body.token.trim();   // trim: stdin newline/whitespace handling by glab is undocumented
// validate: non-empty, <= 512 bytes, no whitespace/control chars
let mut cmd = Command::new("glab");
cmd.args(["auth","login","--hostname",&host,"--stdin","--api-protocol","https","--git-protocol","https","--insecure-storage"]);
apply_glab_env(&mut cmd,&host);
cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
// write token + "\n" to child stdin, drop it, wait_with_output with a ~20s timeout (kill_tree on expiry)
```
- **Never put the token in argv (`-t`), env, a file or a log line.** Pass all glab output through `redact(text, token)` (replace the exact token and any `glpat-[A-Za-z0-9_\-]{10,}`/`gl[a-z]{2,4}-` prefix pattern with `[redacted]`) before it can reach an error string. Also never format the body or `Request` with `{:?}`.
- `serde_json` type errors embed the offending value (`invalid type: string "glpat-..."`); hence the fixed-message `map_err` above. [ASSUMED serde behavior, well known; unit-test it.]
- After login, run the status probe. If it fails or lacks `api`, run `glab auth logout --hostname H` so a bad token is not left stored, then return the classified error.
- Zeroize not available without a crate; just drop the String promptly (shortcut, document).
- Check that `http.rs`/`server` request logging never prints bodies (grep `eprintln!`/`println!` near request handling) and add the redaction test.

### Pattern 5: Status and error classification (AUTH-02)
Run `glab api user` (parse `.username`) and `glab api personal_access_tokens/self` (parse `scopes`, `expires_at`, `active`, `revoked`) with `--hostname H`. Classification on stderr/stdout text + exit code, first match wins:
1. contains `x509`, `certificate`, `tls:` , `no such host`, `connection refused`, `i/o timeout`, `dial tcp` => `unreachable_or_tls` (UI copy cites host + `/run/ocr-ca/ca.crt`)
2. contains `401` => if text contains `expired` => `expired`, else `unauthorized`
3. `personal_access_tokens/self` ok but `revoked` or `!active` or `expires_at` < today => `expired`
4. scopes array lacks `api` => `missing_scope`
5. else `generic` (log classification only, never raw output without redact).
Return a stable machine code (`code`) plus the UI-SPEC message; the frontend can map code -> copy or just display `message`. An expired PAT generally gets a plain 401 on `/user`, so "expired" via text is best effort; the `expires_at` check covers tokens that still answer. These stderr substrings are [ASSUMED]; the spike records the real texts, and the planner should make the matcher a small table that is easy to adjust. `granular_scopes` (GitLab 19.5+) may exist with empty/absent `scopes`: treat absent `scopes` + present `granular_scopes` as "cannot verify scope", do not hard fail [CITED: docs.gitlab.com/api/personal_access_tokens]. `/personal_access_tokens/self` 404 (older instances) => skip scope/expiry (show "No expiry info") rather than error [ASSUMED endpoint age; spike].
Cache the result 30-60s (Settings polls); invalidate on login/logout. Status is derived from glab's store, nothing in the DB.

### Pattern 6: CA bundle (AUTH-04)
`ca::prepare(data_dir)`: if `/run/ocr-ca/ca.crt` is absent or empty => do nothing (public-CA GitLab works). Else read it, require at least one `-----BEGIN CERTIFICATE-----` (else boot Err naming the file), read system bundle `/etc/ssl/certs/ca-certificates.crt` (bookworm; Err if missing), write `system + "\n" + ca` to `$OCR_DATA_DIR/ca-bundle.crt` via temp file + rename, mode 0600, then `std::env::set_var` for `SSL_CERT_FILE`, `GIT_SSL_CAINFO`, `GLAB_CA_CERT` (internal CA only) at the top of `run_from_env` (before threads; edition 2021 so `set_var` is safe). Process-wide is required because `git` is spawned from many places in the codebase. Side effect: `reqwest`/`rustls-native-certs` in the server also honor `SSL_CERT_FILE`; since the file contains system roots too, outbound AI calls are unaffected [ASSUMED: rustls-native-certs honors SSL_CERT_FILE; harmless either way]. Rebuild every boot (picks up CA rotation). No `skip_tls_verify`/`GIT_SSL_NO_VERIFY` anywhere; add a grep test.
Compose (server service only): `- ${OCR_GITLAB_CA_FILE:-/dev/null}:/run/ocr-ca/ca.crt:ro`. Reason: a bind mount of a missing host file makes Docker create a directory at that path; the `/dev/null` default yields an empty file that `prepare` treats as "no CA" [ASSUMED works on Docker Desktop/WSL2; verify in WSL]. Treat "path is a directory" as empty/err with message. Apply the same env + mount to `docker-compose.images.yml` (and check `docker-compose.scale.yml`) and document in `.env.example`/README. Add to server env: `OCR_FORGE: ${OCR_FORGE:-github}`, `OCR_GITLAB_HOST: ${OCR_GITLAB_HOST:-}`. Runner services get none of this.

### Pattern 7: Routes and frontend
- Routes (all behind existing auth; not added to `is_public_route`): `GET /forge/status`, `POST /auth/gitlab/token`, `POST /auth/gitlab/logout`; each POST calls `require_trusted_origin(request)?` like `github_auth_logout`. Content-Type JSON check already global.
- `/health` may also expose `forge` but `/forge/status` is enough.
- Frontend: `src/types/settings.ts` add `ForgeStatus`; `src/services/setupApiAccess.ts` (142 lines) add `forgeStatus`, `gitlabConnect(token)`, `gitlabDisconnect()`; new `GitLabConnection.tsx` per 02-UI-SPEC (copy and states are fixed there). `SettingsPage.tsx` (529 lines, limit 800): add `forgeQuery`; when `provider==="gitlab"` render the panel in the `integrations` section and suppress the `authDisconnected` GitHub banner and GitHub `IntegrationRow`. Token state is local `useState`, cleared on submit; mutation result must not be cached (no `queryClient.setQueryData` with the token; do not put it in the mutation key/variables persisted anywhere; use `mutation.reset()` after). On success invalidate `["forge","status"]`.
- Onboarding/Reviews pages still assume GitHub login (`setupStatus.nextStep`); making those GitLab-aware is beyond FORGE/AUTH requirements; only ensure nothing crashes (GitHub status shows disconnected). Flag as Open Question.

### Anti-Patterns
- Calling `gh_host()` in the guard (shells out, non-deterministic).
- `-t <token>` flag or `GITLAB_TOKEN` env for the UI flow (argv/env leak, and env token overrides stored creds).
- Storing connection state or token in settings table.
- Adding generic `config.yml` to the review-archive deny list (STACK.md suggested): the filter at `review_archive/mod.rs:~396` is by file name over repo content and would drop legitimate repo files; `/data/glab` is not inside repos. Skip.

## Don't Hand-Roll
| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Token store | own encrypted token file/DB column | `glab --insecure-storage` under `GLAB_CONFIG_DIR` | Locked; same threat model as gh `hosts.yml` |
| Host validation | new regex | reuse `is_valid_gh_host` logic (move/copy to `gitlab`) | Prevents token going to wrong host |
| TLS trust | rustls/openssl code, skip-verify | concatenated bundle + env | Covers git, glab, reqwest |
| Process kill on timeout | new code | `crate::review::kill_tree` (used by `run_auth_flow`) | exists |
| Private dir | new fs code | `create_private_dir` | exists |

## Common Pitfalls
1. **Token echo via serde/glab errors** - fixed-message map_err + `redact()` on all child output; test with a fake `glab` script that prints the token to stderr.
2. **Bind mount of missing CA file becomes a directory** - `/dev/null` default (see Pattern 6).
3. **glab keyring on slim image** - without `--insecure-storage` login may fail or hang; always pass it.
4. **glab guesses gitlab.com** - always `--hostname` and `GITLAB_HOST`; strip inherited `GITLAB_TOKEN*`.
5. **Guard flaps on GitHub deploys** - env-only host, never `gh auth status`.
6. **Fresh DB vs existing data** - guard writes defaults then compares, so a first boot never fails; test both orders.
7. **GitHub background workers on GitLab deploys** spam errors - gate them (Pattern 2).
8. **Multi-instance boot race** - `ensure_setting_default` is atomic; compare after read-back.
9. **Env mutation after threads start is UB-ish** - do `set_var` as first thing in `run_from_env`, before `Instance`/pool threads (`Db::from_env` may spawn pool threads: put CA prepare before `ServerConfig::from_env()`).
10. Executor fingerprint hashes `review/*.rs` only; changes in `forge/`, `gitlab/`, `server/` do not alter it. Do not touch `review/*.rs` in this phase.

## Code Examples
Dockerfile block (shape):
```dockerfile
ARG GLAB_VERSION=1.128.0
ARG GLAB_SHA256_AMD64=bc024819e7598b19429a199cd87f640d98b2b67416d8ad796f19e4260b57a7c7
ARG GLAB_SHA256_ARM64=60d0512085a8c1be7c5ef51aa7e4ee28213444cea0376b3260707d90489199ea
RUN case "$TARGETARCH" in amd64) arch=amd64; sum="$GLAB_SHA256_AMD64";; arm64) arch=arm64; sum="$GLAB_SHA256_ARM64";; *) exit 1;; esac \
  && curl -fsSL --retry 3 "https://gitlab.com/gitlab-org/cli/-/releases/v${GLAB_VERSION}/downloads/glab_${GLAB_VERSION}_linux_${arch}.tar.gz" -o /tmp/glab.tgz \
  && printf '%s  /tmp/glab.tgz\n' "$sum" | sha256sum -c - \
  && tar -xzf /tmp/glab.tgz -C /tmp bin/glab && install -m0755 /tmp/bin/glab /usr/local/bin/glab && rm -rf /tmp/glab.tgz /tmp/bin \
  && glab --version | grep -q "$GLAB_VERSION"
```
`TARGETARCH` ARG must be in scope (it is declared before the Claude block; ARGs persist within the stage).

Bundle builder (pure, testable): `fn build_bundle(system: &str, extra: &str) -> Result<String,String>` rejecting `extra` without a PEM CERTIFICATE header.

## Spike deliverable (success criterion 1; non-blocking)
No GitLab is reachable here. Plan one task producing:
- `docker/gitlab-spike.py` (Python 3 stdlib only, run `python3 -I`): inputs `--host`, `--ca PATH` (optional), token from env `GITLAB_TOKEN` (never argv). Uses `ssl.create_default_context(cafile=...)` + `urllib`: GET `/api/v4/version`, `/user`, `/personal_access_tokens/self`, `/projects?membership=true&per_page=1` (record `X-Next-Page`, `X-Total`, `Link` headers), `/merge_requests?state=opened&scope=all&per_page=1`; prints token prefix shape only (`glpat-` + length, never the value); writes `gitlab-spike-result.json`. Optional `--with-glab`: runs the glab probes in an isolated temp `GLAB_CONFIG_DIR`: `glab auth login --stdin --insecure-storage`, `glab auth status --show-token --output json` (record JSON shape), `glab api --paginate` vs `--paginate --output ndjson` on a 2-page list, error text for wrong CA / bad token / expired token, trailing-newline stdin behavior, resulting `config.yml` layout (keys only, token masked), whether `SSL_CERT_FILE` alone makes glab trust the CA vs needing `GLAB_CA_CERT`.
- `.planning/phases/02-.../SPIKE.md`: table of open questions with `Finding:` / `Date:` blank fields: PAT format and length; `/personal_access_tokens/self` availability + fields; GitLab version (`/version`) vs 16.0 floor; pagination headers + glab paginate JSON merge semantics; MR ref `refs/merge-requests/<iid>/head` via `git ls-remote`; CA handling by glab/git; exact error strings for classification; login `--stdin` newline behavior; whether glab login validates the token itself.
Success criterion 1 is satisfied by the delivered artifacts + recorded "pending user run" status; decisions continue per this research.

## State of the Art / Corrections
| Item | Finding |
|------|---------|
| glab CA env | `GLAB_CA_CERT`/`CA_CERT` documented (STACK.md said none) [CITED: docs.gitlab.com/cli/configuration] |
| glab no-prompt | `NO_PROMPT` is an alias of `GLAB_NO_PROMPT`; both fine |
| Config precedence | `$GLAB_CONFIG_DIR/config.yml` first [CITED: same] |
| Token validation at login | docs silent; always verify with `glab api user` ourselves |

## Assumptions Log
| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | glab tarball contains `bin/glab` | Stack | Dockerfile build fails; fix path (caught at build) |
| A2 | Error substrings (x509, 401, expired, dial tcp) | Pattern 5 | Wrong UI message; falls to generic; spike corrects |
| A3 | `--stdin` + `--insecure-storage` works headless without keyring | Pattern 4 | Login fails in container; check in WSL docker smoke |
| A4 | `/personal_access_tokens/self` exists on target instance | Pattern 5 | Skip scope check gracefully (handled) |
| A5 | `/dev/null` bind mount default works on user's Docker | Pattern 6 | Use compose override; verify in WSL |
| A6 | `rustls-native-certs` honors `SSL_CERT_FILE` | Pattern 6 | None (bundle contains roots) |
| A7 | serde_json echoes values in type errors | Pattern 4 | Fixed message makes this moot |
| A8 | main_server prints `Err` and exits non-zero | Pattern 1 | Planner reads file; add print if not |

## Open Questions
1. Onboarding/Reviews UI on GitLab deploys still call GitHub status. Recommendation: out of scope for Phase 2; ensure no crash, handle in Phase 3.
2. Allow `host:port`? Recommend yes (reuse validator); `--hostname host:port` is accepted by glab [ASSUMED].

## Environment Availability
| Dependency | Required By | Available | Notes |
|------------|------------|-----------|-------|
| Docker | image build, smoke | WSL Ubuntu only | build/test via `wsl` |
| cargo / Rust | backend | WSL only | `~/rustcheck.sh`, `~/rusttest.sh` |
| Node/pnpm | frontend | Windows (Node 25) | 20 pre-existing `localStorage.clear()` failures are baseline |
| Real GitLab | spike | No | script delivered, user runs later |
| glab binary locally | tests | No | unit tests use a fake `glab` shell script on PATH (WSL) |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (Postgres via `OCR_TEST_DATABASE_URL`), Vitest 4.1 + jsdom |
| Config | existing; no new framework |
| Quick run | `wsl -e bash -lc '~/rustcheck.sh "cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings"'`; `npx -y pnpm@10.15.0 test -- GitLabConnection` |
| Full suite | `wsl -e bash -lc '~/rusttest.sh'`; `npx -y pnpm@10.15.0 test`; `node scripts/check-file-sizes.mjs` |

### Phase Requirements -> Test Map
| Req | Behavior | Type | Command / location | Exists |
|-----|----------|------|--------------------|--------|
| FORGE-01 | pure guard: same ok, provider/host change => Err naming value; GitLab without host => Err; invalid host rejected | unit | `forge/guard.rs` tests | Wave 0 |
| FORGE-01 | first boot writes keys; second boot mismatch errs; GitHub legacy DB without keys passes | pg test | `db`/`forge` tests with `OCR_TEST_DATABASE_URL` | Wave 0 |
| FORGE-01 | `forge_provider` not in `ALLOWED_SETTING_KEYS` (POST /settings rejects) | unit | db settings test | Wave 0 |
| AUTH-01/03 | login/status/logout against fake `glab` script (PATH override; records args+stdin, prints canned JSON) | unit | `gitlab/auth.rs` tests | Wave 0 |
| AUTH-02 | classifier table: missing api, expired (expires_at past / "expired" text), 401, x509, dial tcp, generic | unit | `gitlab/auth.rs` tests | Wave 0 |
| AUTH-05 | token never in argv/env of fake glab; stderr echoing token is redacted; invalid JSON body error lacks token | unit | `gitlab/auth.rs` tests | Wave 0 |
| AUTH-04 | `build_bundle` concatenates and rejects non-PEM; no `skip_tls_verify`/`GIT_SSL_NO_VERIFY`/`insecure-skip` string in `backend/src`, `docker`, compose | unit + grep test | `gitlab/ca.rs` tests | Wave 0 |
| AUTH-05 | runner image/compose: `Dockerfile.runner` has no `glab`; runner services have no `OCR_GITLAB`/`ca.crt`; server Dockerfile has pinned sha256 | static check (python or rust test reading files) | `docker/test_diagnostics.py` style or rust test | Wave 0 |
| AUTH-01..03 | panel states, token cleared after submit, no token in query cache/localStorage, hidden on GitHub deploy | vitest | `GitLabConnection.test.tsx`, `SettingsPage.test.tsx` | Wave 0 |
| AUTH-04 | real glab/git trust a throwaway-CA TLS server | manual/WSL docker smoke (optional, not gating) | script in `docker/` | deferred |

### Sampling Rate
- Per task: rustcheck (fmt+clippy) for backend tasks; relevant vitest file for frontend tasks.
- Per wave: `rusttest.sh` + full pnpm test + `check-file-sizes.mjs`.
- Phase gate: all green except the documented 20-failure Node 25 baseline; docker build of server image in WSL verifies sha256 + `glab --version` (if time allows).

### Wave 0 Gaps
- Fake-glab helper (shell script written to a tempdir, prepended to `PATH`; tests are `#[cfg(unix)]`, so they run in WSL only).
- Make `ensure_setting_default` reachable from `forge/guard` (visibility only).

## Security Domain
| ASVS | Applies | Control |
|------|---------|---------|
| V2 Authentication | yes | PAT pass-through to glab stdin; no token storage by app |
| V4 Access Control | yes | Routes behind `OCR_API_TOKEN` check + `require_trusted_origin`; guard keys not API-writable |
| V5 Input Validation | yes | host validator, token charset/length, fixed-message body errors |
| V6 Cryptography | yes | no custom crypto; TLS via system+internal CA, no skip-verify |
| V7 Logging | yes | redaction of token and `gl*-` patterns; never log bodies |

| Threat | STRIDE | Mitigation |
|--------|--------|------------|
| Token to wrong host (gitlab.com guess) | Info disclosure | explicit `--hostname`, `GITLAB_HOST`, validated host |
| Token in argv/env/proc list | Info disclosure | stdin only |
| Token in error response | Info disclosure | redact + fixed errors |
| CSRF connecting attacker token | Tampering | origin check + JSON content-type |
| MITM on internal CA | Spoofing | verify always on; bundle file 0600 in data volume |
| Runner obtaining token/CA | Elevation | server-only Dockerfile/compose; static test |

## Sources
- [CITED] https://docs.gitlab.com/cli/auth/login/ - flags, `--stdin`, `--insecure-storage`, storage behavior
- [CITED] https://docs.gitlab.com/cli/configuration/ - env vars incl. `GLAB_CA_CERT`, `GLAB_CONFIG_DIR`, precedence
- [CITED] https://docs.gitlab.com/api/personal_access_tokens/ - `/personal_access_tokens/self` fields
- [VERIFIED] https://gitlab.com/gitlab-org/cli/-/releases/v1.128.0/downloads/checksums.txt - linux amd64/arm64 sha256
- [VERIFIED: codebase] `forge/mod.rs`, `github/auth.rs`, `server/routes.rs`, `server/mod.rs`, `db/settings.rs`, `docker/Dockerfile`, `docker-compose.yml`, `SettingsPage.tsx`, `setupApi*.ts`
- Project: `.planning/research/STACK.md`, `PITFALLS.md`

## Metadata
**Confidence:** stack/Dockerfile HIGH; guard/routes/frontend integration HIGH (read code); glab runtime behavior and error strings LOW-MEDIUM until spike.
**Research date:** 2026-11-05. **Valid until:** ~2026-12-05 (glab releases frequently; pinned version avoids drift).
