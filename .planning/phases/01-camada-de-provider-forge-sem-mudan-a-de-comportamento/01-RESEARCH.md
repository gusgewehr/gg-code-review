# Phase 1: Camada de provider (Forge) - Research

**Researched:** 2026-11-05
**Domain:** Rust refactor (seam over `backend/src/github/`), no behavior change
**Confidence:** HIGH (all facts from grep of the repo this session)

<user_constraints>
## User Constraints (from CONTEXT.md)
- Novo modulo `backend/src/forge/` com enum `Forge { GitHub }` (dispatch por `match`), nao `dyn Trait`.
- Re-exportar tipos GitHub via `forge::` em vez de duplicar (menor diff).
- Selecao do forge vem de `ServerConfig` (sempre GitHub por ora); `OCR_FORGE`/guard de host -> Fase 2.
- URLs `github.com` hard-coded e refspec `refs/pull/<n>/head` passam a vir do forge, bytes identicos.
- Regra `full_name` de dois segmentos fica como esta (Fase 3).
- Gate: `cargo test --workspace --locked`, clippy `-D warnings`, `pnpm test`, `pnpm check:file-size` sem alterar asserções; `grep -rn "github::" backend/src` fora de `github/` e `forge/` = zero.
- Deferred: `OCR_FORGE`/`OCR_GITLAB_HOST` (Fase 2), namespaces aninhados (Fase 3).
</user_constraints>

<phase_requirements>
| ID | Description | Research Support |
|----|-------------|------------------|
| FORGE-02 | Codigo fora de `github/` fala com interface de provider | Inventario abaixo; abordagem re-export + enum |
</phase_requirements>

## Summary

Todo o uso externo e por caminho `github::X` (via `use crate::{github, ...}` em `review/mod.rs:67` e `server/mod.rs:17`, que propaga por `use super::*`) ou `crate::github::X` completo. Quase tudo sao funcoes livres e tipos que podem migrar so trocando o prefixo para `forge::` com um glob re-export. Apenas as URLs e o refspec precisam de metodos novos no `Forge`.

**Primary recommendation:** `forge/mod.rs` com `pub(crate) use crate::github::*;` + `enum Forge { GitHub }` com 3 metodos (`web_host`, `pr_refspec`/`review_url`...). Depois um `sed` mecanico `github::` -> `forge::` nos arquivos listados e troca de `use crate::{github,` por `use crate::{forge,`. Adicionar `pub mod forge;` (ou `mod forge;`) em `backend/src/lib.rs` (hoje `pub mod github;` na linha 4; `mod review;` l.9, `pub mod server;` l.12).

## Architectural Responsibility Map
| Capability | Tier | Rationale |
|---|---|---|
| Hosting API/auth/urls | Backend `forge/` -> `github/` | seam unico |
| Review (sandbox) | unchanged | runner nunca recebe credenciais |

## Inventario de `github::` fora de `backend/src/github/` (grep, linhas atuais)

Todas sao **funcoes livres ou tipos**; todas migram trocando o prefixo (re-export), nenhuma exige metodo de dispatch, exceto os marcados URL/refspec.

- `lib.rs:4` `pub mod github;` (manter; `forge` fica ao lado)
- `db/review_requests.rs:7,17,130` tipo `ReviewRequestSnapshot` (usa `crate::github::`)
- `indexer/mod.rs:620` `apply_git_https_auth`; `indexer/tests.rs:423` e so string de fixture (ignorar)
- `review/mod.rs:67` `use crate::{github, indexer};` (importa o modulo para o submodulos via `use super::*`)
- `review/dependencies.rs:422,1027` `gh_capture_cancellable`
- `review/execution.rs:22` `live_pr_state_cancellable`
- `review/frozen.rs:37` `gh_host`
- `review/git.rs:458` `apply_git_https_auth`
- `review/tests/usage.rs:777,1150` `gh_host`
- `review_archive/mod.rs:268` `is_valid_gh_host`
- `review_archive/retention.rs:462` `gh_capture_cancellable`; `:491` `iso8601_to_epoch_ms`
- `server/mod.rs:17` `use crate::{github, indexer, local, review};`
- `server/comment_activation.rs:28,29,39,40` tipo `ConversationComment`; `:110` `conversation_comments_page`; `:111` `unedited_author_command`
- `server/comment_activation_tests.rs:48,76` `ConversationComment`; `:106` `iso8601_to_epoch_ms`; `:401,506` `GhPullRequest`
- `server/indexing.rs:439` `crate::github::gh_host()` (monta `https://{host}/{full_name}.git`)
- `server/polling.rs:109,121,145,154,176,235,296,300` `list_repos`, `current_login`, `list_open_prs`, `activation_pr`, `live_pr_state`, `list_open_prs_for_login`; tipo `GhPullRequest` `:272,296`
- `server/polling_pr.rs:12,47,102,172,185` `pr_file_paths`, `GhPullRequest`, `iso8601_to_epoch_ms`, `avatar_url`
- `server/publication.rs:41,734` `live_pr_state`; `:395` `GitHubStatusComments`; `:1026,1028,1032,1033` `snapshot_review_requests`, `prepare_review`, `post_review`, `find_review_id_by_marker`; `:1261` `diff_anchors`; `:1323` `dedupe_review_comments`; `:1356` tipo `DedupeResult`
- `server/queue_acknowledgements.rs:2` `use crate::github::StatusCommentApi;` `:99` `GitHubStatusComments`
- `server/response.rs:6` `apply_gh_host`
- `server/review_http.rs:73` `current_login`; `:148,156,162,163` snapshot/prepare/post/find; `:291` tipo `LivePrState`
- `server/review_jobs.rs:136` `live_pr_state_cancellable`; `:165` `pr_description`
- `server/routes.rs:161` `auth_status`; `:166` `sign_out`; `:223` `run_auth_flow`; `:320,326` `list_workspaces`, `list_repos`
- `server/runtime.rs:181` `GitHubStatusComments`; `:292,434` `find_review_id_by_marker`; `:442` `restore_review_requests`
- `server/status_comments.rs:2` `use crate::github::StatusCommentApi;` `:189` `GitHubStatusComments`
- `server/status_comments_test_support.rs:1` `use crate::github::{StatusComment, StatusCommentApi};`
- `server/summary.rs:70` `upsert_issue_comment_by_marker`; `:245,263` `apply_git_https_auth`
- `server/tests.rs:411,418,422` `LivePrState`; `:899,917` `DedupeResult`; `:1115` `PreparedReview::in_directory`
- `server/workspace.rs:13` `list_repos`

Nao ha uso em `main_*.rs`, `bench/`, `accounts/` (CONTEXT dizia accounts; grep nao acha `github::` la). Impl de trait: `StatusCommentApi` e trait + `GitHubStatusComments` impl — re-exportam normalmente (glob re-export cobre traits/structs/fns `pub`/`pub(crate)`).

## URLs e refspec hard-coded (precisam de metodo no Forge)
- `review/git.rs:22` `format!("refs/pull/{number}/head:refs/ocr/pr-{number}")`
- `server/summary.rs:249` `format!("refs/pull/{number}/head:refs/ocr/summary-{number}")`
- `server/publication.rs:120` `https://github.com/{full_name}/commit/{sha}`; `:703` `https://github.com/{}/pull/{}#pullrequestreview-{id}`; `:1152` e `:1229` `https://github.com/{full_name}/commit/{head_sha}`
- `server/review_http.rs:204` `https://github.com/{full_name}/pull/{number}#pullrequestreview-{id}`
- `server/runtime.rs:335` idem pull/review URL
- `server/comment_activation.rs:8` ja usa `https://{host}/{full_name}/commit/{head}` (host de `gh_host()`)
- `server/mod.rs:62` `OCR_REPOSITORY_URL` e link do produto — NAO mexer.
- Fora de escopo (nao sao forge): `sandbox/egress.rs:294`, `sandbox/mod.rs:99`, `bench/reviewbench.rs:197`, fixtures de teste.

Cuidado byte-identico: publication/review_http/runtime usam literal `github.com`, enquanto comment_activation usa `gh_host()`. Para `Forge::GitHub` metodos devem devolver exatamente `github.com` (nao `gh_host()`, que pode ser GHES) nos pontos que hoje sao literais. Recomendado: metodos `Forge::commit_url(full_name, sha)`, `Forge::review_url(full_name, number, id)` e `Forge::pr_head_refspec(number, local_ref_suffix)` / ou `pr_head_ref(number)` retornando `refs/pull/{number}/head`; o chamador concatena `:refs/ocr/...`. Chamadores usam `Forge::GitHub` direto (ou `forge::current()`); ligar a `ServerConfig` e opcional nesta fase (CONTEXT pede, mas o menor diff e `ServerConfig.forge: Forge` com `Forge::GitHub` em `from_env`, `server/mod.rs:71-110`; atencao a outros construtores literais de `ServerConfig` em testes — grep `ServerConfig {` antes). Sugestao: como nao ha como o runtime ler o config em fns livres, use `Forge::default()`/const `Forge::GitHub` e adicione o campo so se barato.

## Abordagem minima recomendada
1. `backend/src/forge/mod.rs`: `pub(crate) use crate::github::*;` (se `unused_imports` reclamar de glob, `#[allow(unused_imports)]`) + `#[derive(Clone, Copy, Default)] pub(crate) enum Forge { #[default] GitHub }` com os 3 metodos acima (match de um braco).
2. `lib.rs`: `mod forge;` (pub se testes de integracao precisarem; github e `pub`).
3. `sed -i 's/\bgithub::/forge::/g'` nos arquivos listados (incluindo `crate::github::` -> `crate::forge::`); trocar `use crate::{github, ...}` por `use crate::{forge, ...}` em `review/mod.rs:67` e `server/mod.rs:17`. Rever diff: nao tocar `indexer/tests.rs:423` (string), nem `server/comment_activation.rs:8` string literal.
4. Trocar os literais de URL/refspec pelos metodos.
5. Nao tocar `review/*.rs` mais que isso: mudar esses arquivos muda o fingerprint `sandbox::engine_version` (`sandbox/mod.rs:24-33` hasheia `review/mod.rs`, `evidence.rs`, `warnings.rs`, `dependencies.rs` ...). `review/mod.rs` e `review/dependencies.rs` entram no hash — aceitavel (backend e runner rebuildados juntos), mas e a razao para os smokes Docker rebuildarem imagens.

Headroom: maiores tocados `server/publication.rs` 1444, `review/execution.rs` 1198, `review/dependencies.rs` 1191, todos bem abaixo de 2500; mudanca e neutra em linhas. `forge/mod.rs` novo, pequeno. `github/` tem 2628 linhas somados, nenhum arquivo perto do limite.

## Don't Hand-Roll
Nao duplicar tipos (`GhPullRequest`, `LivePrState`, ...) em tipos neutros; nao criar trait. Nao reimplementar logica de `gh`.

## Common Pitfalls
- **Glob re-export nao cobre itens privados**: se alguma fn em `github/` for `pub(super)`/privada e usada fora, o compilador acusa; subir para `pub(crate)` ou re-export explicito.
- **sed pega strings/comentarios**: revisar `git diff` para strings contendo `github::`.
- **`use super::*`** em `server/` e `review/` depende de `forge` estar importado no `mod.rs` pai; submodulos de teste (`server/tests.rs`, `review/tests/usage.rs` usa `crate::github::` completo).
- **Fingerprint**: nao e bug, so rebuild das duas imagens.
- **Gate grep** pega `github::` dentro de `forge/` ? Excluido pelo CONTEXT; mas `forge/mod.rs` com `crate::github::*` e permitido.

## Environment Availability
| Dependency | Available | Notes |
|---|---|---|
| cargo / rustc | NO (`command not found` no Git Bash) | pode estar no PowerShell PATH; nao verificado la. Se nao, nada compila localmente |
| pnpm | NO (Git Bash) | idem |
| docker | NO (Git Bash) | smokes nao rodam localmente |
| OCR_TEST_DATABASE_URL | vazio | testes Rust com DB nao rodam localmente |

Planner: incluir primeira tarefa verificando `cargo --version` no PowerShell; se ausente, validacao fica por grep + revisao e CI. [VERIFIED: bash probes this session]

## Validation Architecture
| Property | Value |
|---|---|
| Rust | `cargo test --workspace --locked` (precisa `OCR_TEST_DATABASE_URL`), `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --check` |
| Frontend | `pnpm test`, `pnpm check:file-size` (frontend inalterado; so file-size cobre Rust tambem) |
| Quick compile | `cargo check --workspace --locked --all-targets` |
| Grep gate | `grep -rnE "github::" backend/src --include=*.rs \| grep -v "^backend/src/github/" \| grep -v "^backend/src/forge/"` deve ser vazio; ainda `grep -rnE "use crate::\{?github|super::github" backend/src \| grep -v "^backend/src/\(github\|forge\)/"` vazio; e `grep -rn "https://github.com/{" backend/src/server` so em `OCR_REPOSITORY_URL`/vazio |
| Nao roda localmente | cargo/pnpm/docker/Postgres (conforme probes); smokes Docker e testes com DB dependem de CI/outra maquina |

Teste novo opcional pequeno: unit test em `forge/mod.rs` afirmando `Forge::GitHub.commit_url("a/b","abc") == "https://github.com/a/b/commit/abc"` e refspec identico — protege a equivalencia byte a byte.

## Security Domain
Sem mudanca de superficie: nenhum token novo, nenhum log novo. `apply_git_https_auth`/`apply_gh_host` continuam so no server (runners sem credenciais). ASVS: n/a alem de nao regredir V7 (nao logar credenciais).

## Assumptions Log
| # | Claim | Risk |
|---|---|---|
| A1 | Itens em `github/` usados fora sao `pub`/`pub(crate)` (glob re-export cobre) | compile error simples de corrigir |
| A2 | cargo/pnpm podem existir fora do Git Bash (PowerShell) | nao verificado |

## Sources
- Repo grep/probes desta sessao (HIGH); `CLAUDE.md`, `01-CONTEXT.md`.

**Research date:** 2026-11-05 | **Valid until:** ate o codigo mudar
