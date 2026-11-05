# Phase 1: Camada de provider (Forge) sem mudança de comportamento - Context

**Gathered:** 2026-11-05
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous — answers chosen by Claude, user delegated all decisions)

<domain>
## Phase Boundary

Código fora de `backend/src/github/` passa a falar com uma camada de provider (`backend/src/forge/`), e o fluxo GitHub continua byte-idêntico. Nada de GitLab nesta fase além do que for necessário para a interface existir.

</domain>

<decisions>
## Implementation Decisions

### Forma da camada
- Módulo novo `backend/src/forge/` com um enum `Forge { GitHub }` (dispatch por `match`), não `dyn Trait` — mais simples no código síncrono atual e fácil de estender com `GitLab` na Fase 2.
- Tipos neutros só onde já existe tipo GitHub atravessando a fronteira; quando o tipo GitHub já serve, re-exportar via `forge::` em vez de duplicar (menor diff).
- Seleção do forge vem de `ServerConfig` (por enquanto sempre GitHub); `OCR_FORGE`/guarda de host ficam para a Fase 2.
- `FakeForge` para testes só se algum teste novo precisar; não é requisito.

### Pontos neutralizados já nesta fase
- URLs `github.com` hard-coded e refspec `refs/pull/<n>/head` (`review/git.rs`, `server/summary.rs`) passam a vir do forge.
- Regra de `full_name` com exatamente dois segmentos (`Scope::validate`, retenção `split_once('/')`) fica como está nesta fase se mexer mudar comportamento; afrouxar fica para a Fase 3 (FORGE-03).

### Verificação
- Gate: `cargo test --workspace --locked`, `cargo clippy -D warnings`, `pnpm test`, `pnpm check:file-size` verdes sem alterar asserções existentes.
- Gate: `grep -rn "github::" backend/src` fora de `backend/src/github/` e `backend/src/forge/` retorna zero.
- Smokes Docker são desejáveis mas não bloqueiam se o ambiente local não rodar Docker.

### Claude's Discretion
Todo o resto (nomes, divisão de arquivos, ordem das migrações de call sites) fica a critério do planner/executor, priorizando o menor diff que cumpra o gate. Usuário pediu velocidade acima de qualidade.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `backend/src/github/` (gh CLI wrapper: `prepare_review`, `list_open_prs`, `list_repos`, `list_workspaces`, `diff_anchors`, `IssueCommentUpsert`, `gh_host`, `current_login`, `sign_out`).

### Established Patterns
- Rust síncrono, `Result<T, String>` em handlers, `use super::*;` em `server/` e `review/`, re-exports `pub(crate) use`.
- Rust ≤ 2.500 linhas por arquivo; `publication.rs`, `chunks.rs` perto do limite — código novo em módulos novos.

### Integration Points
- ~26 arquivos chamam `github::` diretamente (server/polling*, publication, review_jobs, summary, review/git.rs, indexer, accounts).
- Executor fingerprint (`sandbox::engine_version`) hasheia `review/*.rs` — mudar esses arquivos muda o fingerprint, ok desde que backend e runner sejam rebuildados juntos.

</code_context>

<specifics>
## Specific Ideas

Pesquisa em `.planning/research/` (ARCHITECTURE.md, SUMMARY.md) é a referência principal para o desenho do seam.

</specifics>

<deferred>
## Deferred Ideas

- Provider guard `OCR_FORGE`/`OCR_GITLAB_HOST` → Fase 2.
- Namespaces aninhados (`grupo/subgrupo/projeto`) → Fase 3.

</deferred>
