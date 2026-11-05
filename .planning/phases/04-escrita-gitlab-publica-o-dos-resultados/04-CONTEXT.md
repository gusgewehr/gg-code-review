# Phase 4: Escrita GitLab: publicação dos resultados - Context

**Gathered:** 2026-11-05
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous — answers chosen by Claude, user delegated all decisions)

<domain>
## Phase Boundary

Publicar a revisão validada num MR GitLab: discussões inline, fallback para nota geral, nota de status editada no lugar, nota de resumo, idempotência por marcador, abort em head desatualizado e texto neutralizado. Nunca aprovar, mergear ou alterar o MR.

</domain>

<decisions>
## Implementation Decisions

### Inline (PUB-01/02)
- Antes de postar: `GET projects/:id/merge_requests/:iid/versions`, usar a versão mais recente (`base_commit_sha`, `start_commit_sha`, `head_commit_sha`) para `position`; nunca SHAs do git local.
- Linha adicionada: só `new_line`; removida: só `old_line`; contexto: ambos. `old_path` e `new_path` sempre. `position_type=text`.
- `POST .../discussions` com corpo JSON via `glab api --input <arquivo>`, nunca `-f/-F`. Conferir que a nota criada tem `type == "DiffNote"`; se não, ou se vier erro 400 de posição, postar como nota geral com o mesmo marcador `<!-- ocr-... -->` e o caminho/linha no texto.
- Uma linha por finding (multi-linha `line_range` fica para v2).

### Status e resumo (PUB-03/04)
- Nota de status: procurar nota com o marcador de status (`GET .../notes`, ler `body`, não `body_html`); se existe, `PUT` no lugar; senão cria. Mesmo texto do GitHub.
- Resumo: nota geral com o marcador da revisão, postada por último; o id dela é o "review id" para idempotência. Seção de sugestões quando Post suggestions está ligado, igual ao GitHub.
- A impl GitLab devolve ids/estados no formato que `publication.rs` já espera (ids BIGINT), para não ramificar lá; código novo em módulo novo.

### Idempotência (PUB-05)
- Antes de postar, listar discussões/notas existentes do MR e pular findings cujo marcador já existe. Retry após falha parcial reposta só o que falta.

### Head e segurança do texto (PUB-06/07)
- Re-ler o MR (`sha`) logo antes de postar; se diferente do head revisado, abortar com erro claro (mesma semântica do GitHub).
- Pós-processador do corpo para GitLab: linhas que começam com `/` não podem virar quick action (prefixar com zero-width space); `@user`, `#123`, `!123`, `~label`, `%milestone` neutralizados com zero-width space após o sigilo; blocos de código continuam legíveis; fence `suggestion` vira `diff`. Teste unitário por caso.
- Truncar corpo no limite do GitLab (1.000.000 caracteres) com aviso.

### Claude's Discretion
Nomes, layout de `backend/src/gitlab/publish*.rs`, forma exata do escape. Velocidade acima de polimento.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `backend/src/server/publication.rs` (1447 linhas; idempotência, seleção validada), `IssueCommentUpsert`, `diff_anchors`.
- Marcadores `<!-- ocr-... -->` já usados no fluxo GitHub.

### Established Patterns
- Publicação idempotente por fingerprint/marcador; upsert da nota de status.

### Integration Points
- Dispatch do forge nas funções de publicação; GitLab devolve tipos no formato GitHub.

</code_context>

<specifics>
## Specific Ideas

Pesquisa: `.planning/research/PITFALLS.md` (top 5) e `FEATURES.md`.

</specifics>

<deferred>
## Deferred Ideas

- Multi-linha (`line_range`), sugestões aplicáveis inline, auto-resolve → v2.

</deferred>
