# OpenCodeReview — GitLab self-hosted

## What This Is

Fork do OpenCodeReview (BridgLabs, v0.2.0) que adiciona suporte a GitLab self-hosted. Um deploy configurado para GitLab descobre merge requests por polling, revisa com Claude Code ou Codex no sandbox existente e publica os findings como discussões inline no diff do MR, com a mesma experiência que o produto já oferece para pull requests do GitHub. É para times que hospedam o código num GitLab próprio, começando pelo time do autor.

## Core Value

Um MR aberto numa instância GitLab self-hosted (com CA interna) é revisado e recebe findings inline automaticamente, sem quebrar nada do fluxo GitHub existente.

## Requirements

### Validated

<!-- Inferido do código existente em v0.2.0 (commit 1a47d4f). -->

- ✓ Descoberta de PRs do GitHub por polling (`backend/src/server/polling.rs`, `OCR_POLL_INTERVAL_SECONDS`) — existente
- ✓ Revisão por Claude Code (`claude --print`) ou Codex (`codex exec`) em runners sandboxed via IPC — existente
- ✓ Evidência congelada (review archive) e validação de findings contra o snapshot antes de publicar — existente
- ✓ Publicação idempotente de findings inline, comentário de status e resumo em PRs via `gh` — existente
- ✓ Indexação de repositórios (metadados, grafo de pacotes/imports, símbolos leves) e skills — existente
- ✓ GitHub Enterprise via `OCR_GH_HOST` — existente
- ✓ Settings: Providers, Repository access, PR selection, Integrations; engine picker no top bar — existente

### Active

- [ ] Camada de provider de hospedagem de código (GitHub | GitLab) escolhida por deploy; o código fora de `backend/src/github/` deixa de chamar `github::` diretamente
- [ ] Autenticação no GitLab self-hosted via `glab` CLI (`glab auth login --hostname`), com host configurado por variável (ex.: `OCR_GITLAB_HOST`)
- [ ] Confiança em CA interna/certificado próprio para `glab` e `git fetch` dentro dos containers
- [ ] Listar workspaces (grupos/subgrupos) e projetos GitLab em Repository access
- [ ] Polling de MRs abertos e detecção de novo head, enfileirando revisões como hoje
- [ ] Preparação da revisão com base/head/merge-base vivos do MR via fetch de refs (`refs/merge-requests/<iid>/head`), sem checkout
- [ ] Publicação de findings como discussões inline no diff do MR (posição com base/start/head SHA), idempotente e com os marcadores `<!-- ocr-... -->`
- [ ] Comentário de status e resumo da revisão no MR (notes), incluindo seção de sugestões quando ligada
- [ ] Indexação de repositórios e skills funcionando para projetos GitLab
- [ ] UI mostra termos e links corretos do GitLab (MR, `!iid`, URLs da instância) quando o deploy é GitLab
- [ ] Fluxo GitHub continua passando em todos os testes e smokes existentes

### Out of Scope

- GitHub e GitLab ao mesmo tempo no mesmo deploy — o usuário escolheu um provider por deploy; simplifica schema e UI
- Várias instâncias GitLab por deploy — uma instância por deploy, como `OCR_GH_HOST`
- Webhooks do GitLab — o produto descobre PRs por polling; manter o mesmo modelo
- Tokens gerenciados pelo OpenCodeReview — autenticação delegada ao `glab`, espelhando o `gh`
- GitLab.com SaaS como alvo de teste — o foco é self-hosted (deve funcionar por consequência, mas não é critério de pronto)
- Aprovação automática de MR — invariante do produto: nada aprova automaticamente
- Benchmark (`ocr-bench`) para GitLab — benchmark é independente de hospedagem
- Comentários multi-linha, sugestões inline e auto-resolver na v1 — o fluxo GitHub atual não tem; a v1 é só acesso ao GitLab com paridade

## Context

- Base: clone de https://github.com/BridgLabs/opencodereview em v0.2.0 (commit `1a47d4f`), mapa do código em `.planning/codebase/`.
- Backend Rust síncrono (threads, sem async), servidor HTTP próprio, Postgres; frontend React 19 + TanStack. Tudo roda via `docker compose`.
- O código GitHub vive em `backend/src/github/` (~2,6k linhas, chama `gh`); 26 arquivos fora dele chamam `github::` diretamente. O refspec de fetch é específico do GitHub (`CONCERNS.md`).
- Runners não têm acesso a Git/GitHub nem ao banco; fetch e publicação acontecem no `server`. O egress dos runners só libera hosts de AI, então o GitLab não entra na allowlist dos runners.
- Ambiente-alvo do autor: GitLab self-hosted com HTTPS por CA corporativa interna.
- Licença PolyForm Small Business 1.0.0; upstream tem CLA e regras em `AGENTS.md`/`CONTRIBUTING.md`.

## Constraints

- **Runtime**: só Docker Compose — regra do `AGENTS.md`; `glab` entra na imagem do server com versão fixa e checksum verificado, sem instalação em runtime
- **Segurança**: nenhum token GitLab em código, logs, banco ou browser; runners nunca recebem credenciais de hospedagem
- **Arquitetura**: preservar invariantes de revisão (escopo com refs vivos, fetch falho falha a preparação, evidência congelada, publicação idempotente)
- **Compatibilidade**: zero regressão no fluxo GitHub; migrations novas só no fim da lista, nunca editar as aplicadas
- **Tamanho de arquivo**: Rust ≤ 2.500 linhas, frontend ≤ 800 linhas (`pnpm check:file-size`)
- **Estilo**: Conventional Commits, sem atribuição de AI em commits/PRs, release notes em `changelog.d/`

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| GitLab ao lado do GitHub atrás de uma camada de provider | Manter o produto original funcionando e o diff reaproveitável | — Pending |
| Autenticação via `glab` CLI | Espelha o uso de `gh`; sem tokens gerenciados pelo app | — Pending |
| Um provider por deploy (GitHub ou GitLab) | Escolha do usuário; evita misturar repositórios de providers no schema e na UI | — Pending |
| Uma instância GitLab por deploy via variável de host | Igual a `OCR_GH_HOST`; suficiente para o caso real | — Pending |
| Polling, sem webhooks | Mesmo modelo do produto; não exige expor o server para o GitLab | — Pending |
| Paridade completa na v1 (inline, status, resumo, indexação) | Critério de pronto é um MR real revisado com findings inline | — Pending |
| Login do GitLab por formulário na UI (PAT repassado ao `glab` via stdin) | Escolha do usuário; mais conveniente que o terminal | — Pending |
| Spike contra a instância real antes de construir GitLab | Pesquisa deixou formato de token, paginação, CA e `/versions` sem verificação | — Pending |
| Construir para uso próprio primeiro; PR upstream é bônus | Usuário não quer depender da discussão com a BridgLabs agora | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-11-04 after initialization*
