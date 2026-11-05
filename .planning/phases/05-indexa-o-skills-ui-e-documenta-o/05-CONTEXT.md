# Phase 5: Indexação, skills, UI e documentação - Context

**Gathered:** 2026-11-05
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous — answers chosen by Claude, user delegated all decisions)

<domain>
## Phase Boundary

Paridade final num deploy GitLab: indexação e skills em projetos GitLab, UI com termos e links do GitLab, documentação de configuração e release note.

</domain>

<decisions>
## Implementation Decisions

### Indexação e skills (IDX-01/02)
- Indexação usa o clone/fetch via forge (mesma auth git da Fase 3). Nada específico de GitLab além da clone URL `https://<host>/<path_with_namespace>.git` e do default branch vindo da API.
- Skills: só garantir que nada assume `owner/name` de dois segmentos nem `github.com`.

### UI (UI-01)
- Payload de status/setup ganha `forge` (`github`|`gitlab`) e `forgeHost`; o frontend lê isso uma vez pelo serviço de setup existente.
- Em deploy GitLab: "pull request" vira "merge request", `#123` vira `!123`, marca GitHub vira texto "GitLab" (ícone genérico do lucide, sem logo de marca), links usam URLs vindas do backend.
- Implementação mínima: helper `forgeTerms(forge)` com as strings, usado nos poucos lugares que mostram PR/#. Deploy GitHub fica idêntico.
- Mensagens de erro de sign-in citam `glab`/PAT em deploy GitLab.

### Docs e release (UI-02)
- `docs/gitlab.md` (ou seção no README): `OCR_FORGE`, `OCR_GITLAB_HOST`, montagem da CA, criação do PAT (escopo `api`, usuário bot recomendado), login em Settings, como rodar o script de spike, limitações da v1.
- Fragmento em `changelog.d/` no formato existente.

### Claude's Discretion
Componentes exatos tocados e texto da doc. Velocidade acima de polimento; UI-SPEC pode ser mínimo.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/components/GitHubMark.tsx`, `src/services/setupApi*.ts`, `src/platform/pages/reviews/*`.
- `changelog.d/` e o tooling de changelog em `scripts/`.

### Established Patterns
- Frontend ≤ 800 linhas por arquivo; `ReviewsPage.tsx` e `reviewFlow.ts` perto do limite, então strings novas vão em módulo novo.
- Vitest local roda em Node 25 e tem 20 falhas pré-existentes em `localStorage.clear()` (baseline, não regressão).

### Integration Points
- Endpoint de setup status em `server/routes.rs` e `src/services/setupApi.ts`.

</code_context>

<specifics>
## Specific Ideas

None beyond research.

</specifics>

<deferred>
## Deferred Ideas

- PR upstream para a BridgLabs (v2).

</deferred>
