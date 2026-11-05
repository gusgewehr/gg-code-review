# Phase 2: Conexão com o GitLab self-hosted - Context

**Gathered:** 2026-11-05
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous — answers chosen by Claude, user delegated all decisions)

<domain>
## Phase Boundary

Admin escolhe GitLab como provider do deploy por configuração, monta a CA interna, cola um PAT em Settings e vê host/usuário conectados. `glab` fixo na imagem do server. Nada de listagem de projetos, polling ou publicação (Fases 3 e 4).

</domain>

<decisions>
## Implementation Decisions

### Seleção de provider e guarda (FORGE-01)
- Env `OCR_FORGE=github|gitlab` (default `github`) e `OCR_GITLAB_HOST` (host sem esquema, ex. `gitlab.empresa.local`; obrigatório quando `OCR_FORGE=gitlab`).
- Guarda: na primeira subida grava `forge_provider` e `forge_host` na tabela de settings existente (sem migration nova se settings for chave/valor; se precisar de coluna, migration nova no fim da lista). Subida posterior com valor diferente → server sai com mensagem clara dizendo qual valor mudou e como reverter.
- Deploy GitHub existente sem as chaves gravadas: grava `github` + host atual na primeira subida, sem quebrar.
- `Forge` ganha a variante `GitLab` (enum da Fase 1); operações ainda não implementadas retornam erro claro "not supported yet on GitLab" em vez de panic.

### CA interna (AUTH-04)
- Bundle opcional montado via compose em caminho fixo (ex. `/run/ocr-ca/ca.crt`, volume read-only só no serviço `server`).
- No boot, o server concatena o bundle do sistema + CA interna num arquivo em `OCR_DATA_DIR` e exporta `SSL_CERT_FILE` e `GIT_SSL_CAINFO` para os processos filhos (`glab`, `git`). Nenhuma flag de skip-verify existe.
- Runners não montam a CA nem recebem `glab` ou token (AUTH-05).

### glab na imagem (AUTH-05)
- `glab` 1.128.0 baixado no `docker/Dockerfile` (server) de tarball oficial com sha256 fixo, igual ao padrão de Claude/Codex CLIs. Não entra em `Dockerfile.runner`.
- Env dos filhos: `GLAB_CONFIG_DIR` no volume de dados do server, `GLAB_SEND_TELEMETRY=false`, `GLAB_CHECK_UPDATE=false`, `NO_PROMPT=1`.

### Login por PAT na UI (AUTH-01/02/03)
- Settings ganha um painel GitLab (espelho do bloco GitHub existente, reusando `SettingsControls`) com campo password para o PAT, botão Connect e Disconnect. Mostrado só quando o deploy é GitLab; no deploy GitHub a UI não muda.
- Endpoint `POST` recebe o token no corpo, repassa via stdin para `glab auth login --hostname H --stdin` e nunca o loga, ecoa ou grava no banco. Respostas de erro não contêm o token.
- Status: `glab api user` → username; `glab api personal_access_tokens/self` → escopos e `expires_at`. Erros acionáveis: sem escopo `api`, expirado, 401, host fora do ar/erro TLS (mensagem cita a CA).
- Disconnect: `glab auth logout --hostname H`.
- Teste de redação: string `glpat-...` nunca aparece em logs/respostas.

### Spike contra GitLab real
- Não há acesso à instância real neste ambiente. Entregar `docker/gitlab-spike.py` (ou similar) que o usuário roda contra a instância e um `SPIKE.md` com as perguntas abertas da pesquisa e campos para os achados. Não bloqueia a fase; decisões seguem a pesquisa até o spike confirmar.

### Claude's Discretion
Nomes de endpoints, layout exato do painel, estrutura de módulos em `backend/src/gitlab/`. Velocidade acima de polimento.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `backend/src/github/auth.rs`: `gh_config_dir`, `prepare_gh_config_dir`, `create_private_dir`, `apply_gh_host`, `auth_status`/`GitHubAuthStatus`, `sign_out` — espelhar para glab.
- `src/platform/pages/settings/GitHubAuthRecovery.tsx`, `SettingsControls.tsx`, `ReviewSettingsPanel.tsx`.
- `docker/Dockerfile` já baixa CLIs com checksum.

### Established Patterns
- Comandos externos via `std::process::Command` com env controlado; erros `Result<T, String>` sem segredos.
- Settings do usuário no banco, config de deploy em env `OCR_*` no compose.

### Integration Points
- `server/mod.rs` (`ServerConfig`, `run_from_env`) para guarda e CA no boot.
- `server/routes.rs` para endpoints de login/status/logout GitLab.
- `docker-compose.yml` para `OCR_FORGE`, `OCR_GITLAB_HOST`, mount da CA.

</code_context>

<specifics>
## Specific Ideas

Pesquisa: `.planning/research/STACK.md` (glab, auth, CA) e `PITFALLS.md` (TLS em três trust stores).

</specifics>

<deferred>
## Deferred Ideas

- Confirmação real do spike (formato `--show-token`, paginação, `/versions`) — depende do usuário rodar o script.

</deferred>
