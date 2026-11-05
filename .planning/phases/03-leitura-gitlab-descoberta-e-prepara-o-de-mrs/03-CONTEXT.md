# Phase 3: Leitura GitLab: descoberta e preparação de MRs - Context

**Gathered:** 2026-11-05
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous — answers chosen by Claude, user delegated all decisions)

<domain>
## Phase Boundary

Em deploy GitLab: listar grupos/subgrupos/projetos em Repository access, descobrir MRs abertos por polling, detectar novos heads e preparar a revisão (fetch do ref do MR, diff local, evidência congelada). Nada é postado no GitLab nesta fase (Fase 4).

</domain>

<decisions>
## Implementation Decisions

### Identidade e namespaces (FORGE-03)
- `full_name` de projeto GitLab = `path_with_namespace` (ex. `grupo/sub/proj`). Onde o código parte `owner/name`, usar `rsplit_once('/')`: owner = namespace completo, name = último segmento. Para GitHub (dois segmentos) o resultado é idêntico.
- Validações que exigem exatamente dois segmentos (`Scope::validate`, retenção) aceitam 2 ou mais segmentos não vazios.
- Número do PR = `iid` do MR (por projeto), mesma coluna/tipo do número do PR do GitHub. Sem migration se a coluna já for inteiro.

### Listagem de projetos (REPO-01)
- `glab api --paginate "projects?membership=true&min_access_level=30&archived=false&simple=true&per_page=100"`. Grupos/subgrupos vêm de `namespace.full_path`; a UI agrupa por namespace usando o seletor existente (`RepositorySelector`, workspaces = namespaces).
- Parser de paginação tolerante: aceitar arrays JSON concatenados e NDJSON (o spike confirma qual).

### Polling e head (REPO-02/03/04/05)
- `glab api --paginate "projects/<id>/merge_requests?state=opened&per_page=100"`; head = `sha`; draft = `draft` (fallback `work_in_progress`) respeitando `skip_draft_prs` como `polling_pr.rs` faz no GitHub.
- MRs que somem da lista de abertos (closed/merged) saem da fila pelo mesmo caminho dos PRs fechados.
- Novo head enfileira e supersede reusando `review_jobs.rs`, sem ramificar por provider além do forge.
- 429/5xx: retry com backoff curto; erro persistente vira erro de polling existente, não panic.

### Preparação (REV-01/02/03)
- Fetch `refs/merge-requests/<iid>/head` do projeto alvo (funciona para MRs de fork sem credenciais do fork) + target branch, sem checkout; fetch falho falha a preparação. SHA buscado deve bater com `sha` da API; divergência falha (ref atrasado) e o próximo polling tenta de novo.
- Base viva = tip da target branch; merge-base e diff calculados localmente com git (sem `/changes` nem `/diffs`).
- Git auth: extraheader `Authorization: Basic base64(oauth2:<token>)` via env `GIT_CONFIG_*`, token lido do config do glab (`glab auth status --show-token` ou `config.yml`), nunca em argv nem log.
- URLs vêm do forge: `https://<host>/<ns>/<proj>/-/merge_requests/<iid>` e `/-/commit/<sha>`.

### Claude's Discretion
Estrutura de `backend/src/gitlab/`, nomes, encoding do id do projeto (path URL-encoded ou id numérico). Velocidade acima de polimento; testes unitários com fixtures JSON para os parsers.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `backend/src/github/` (list_workspaces, list_repos, list_open_prs, prepare_review, GhPullRequest): espelhar o formato de retorno para que `server/` não ramifique.
- `backend/src/server/polling.rs`, `polling_pr.rs` (305 linhas), `review_jobs.rs`.
- `src/platform/pages/settings/RepositorySelector.tsx` (87 linhas).

### Established Patterns
- `split_once('/')` em `db/repositories.rs:6`, `github/review_requests.rs:75`, `github/repo.rs:32`, `bench/mod.rs:121`.
- Fase 1 criou `backend/src/forge/` (enum `Forge`, `forge::current()`, `commit_url`/`review_url`/`pr_head_ref`); Fase 2 adiciona a variante GitLab e o módulo `backend/src/gitlab/`.

### Integration Points
- Dispatch do forge nas funções de listagem/polling/prepare.
- `review/git.rs` usa `forge.pr_head_ref`.

</code_context>

<specifics>
## Specific Ideas

Pesquisa: `.planning/research/ARCHITECTURE.md`, `PITFALLS.md` (atraso do MR ref, paginação).

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
