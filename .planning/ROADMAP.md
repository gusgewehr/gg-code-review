# Roadmap: OpenCodeReview — GitLab self-hosted

## Visão geral

Cinco fases, na ordem de risco: primeiro a camada de provider sem mudar o comportamento do GitHub, depois a conexão com a instância GitLab real (incluindo um spike), depois leitura (descoberta e preparação), escrita (publicação) e, por fim, indexação, UI e docs. Cada fase entrega algo verificável contra o GitLab real do autor ou contra os testes existentes do GitHub.

## Fases

- [x] **Phase 1: Camada de provider (Forge) sem mudança de comportamento** - Refatora o acesso ao GitHub atrás de uma interface, sem alterar nada no fluxo atual (completed 2026-11-05)
- [x] **Phase 2: Conexão com o GitLab self-hosted** - Seleção de provider por deploy, `glab` fixo na imagem do server, CA interna e login por PAT na UI (completed 2026-11-05)
- [ ] **Phase 3: Leitura GitLab: descoberta e preparação de MRs** - Lista projetos, descobre MRs por polling e prepara a revisão com refs vivos
- [ ] **Phase 4: Escrita GitLab: publicação dos resultados** - Findings inline, nota de status, resumo, idempotência e segurança do texto
- [ ] **Phase 5: Indexação, skills, UI e documentação** - Paridade final: indexação, skills, termos do GitLab na UI, docs e release note

## Detalhes das fases

### Phase 1: Camada de provider (Forge) sem mudança de comportamento

**Goal**: O código fora de `backend/src/github/` passa a falar com uma interface de provider, e o fluxo GitHub continua exatamente igual
**Depends on**: Nada (primeira fase)
**Requirements**: FORGE-02
**Success Criteria** (o que deve ser verdade):

  1. `cargo test --workspace --locked` e `pnpm test` passam sem alterar nenhuma asserção de teste existente
  2. Os smokes existentes (`docker/runner-smoke.py` e demais scripts de Docker) passam sem mudanças
  3. Um PR do GitHub é revisado e publicado com corpos de comentário e URLs idênticos aos de antes da refatoração
  4. Nenhum arquivo fora de `backend/src/github/` e da camada de provider chama `github::` diretamente

**Plans**: 1 plan

Plans:

- [x] 01-01-PLAN.md — forge seam (re-export + enum Forge { GitHub }), github:: -> forge:: rewrite, URL/refspec via Forge methods

### Phase 2: Conexão com o GitLab self-hosted

**Goal**: O admin escolhe GitLab como provider do deploy e conecta à instância interna (CA própria) colando um PAT, sem expor o token
**Depends on**: Fase 1
**Requirements**: FORGE-01, AUTH-01, AUTH-02, AUTH-03, AUTH-04, AUTH-05
**Success Criteria** (o que deve ser verdade):

  1. Um spike contra a instância GitLab real confirma formato do PAT, paginação, CA e versão da API, e os achados ficam registrados antes de construir o resto
  2. O admin define o provider e o host por configuração; o server se recusa a iniciar, com mensagem clara, se provider ou host mudarem sobre dados existentes
  3. O admin cola um PAT em Settings e vê o status da conexão (host e usuário); token sem escopo `api`, expirado ou host fora do ar mostram erro acionável
  4. O admin consegue desconectar o GitLab; o token nunca aparece no browser, nos logs nem no banco
  5. `glab` e `git` do server confiam na CA interna montada, sem opção de desligar a verificação TLS; a imagem dos runners não contém `glab`, token nem CA

**Plans**: 4 plans

Plans:
**Wave 1**

- [x] 02-01-PLAN.md — forge selection from env + pin guard in settings, Forge::GitLab gating, internal CA bundle at boot
- [x] 02-02-PLAN.md — glab pinned in server image, compose env + CA mount (server only), isolation static check, changelog, spike kit
- [x] 02-03-PLAN.md — Settings GitLab connection panel + /forge/status branching (frontend)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 02-04-PLAN.md — glab login/status/logout with token on stdin, error classification, redaction, HTTP routes

**UI hint**: yes

### Phase 3: Leitura GitLab: descoberta e preparação de MRs

**Goal**: MRs abertos dos projetos GitLab escolhidos aparecem na lista de revisões e têm a revisão preparada com evidência congelada correta
**Depends on**: Fase 2
**Requirements**: FORGE-03, REPO-01, REPO-02, REPO-03, REPO-04, REPO-05, REV-01, REV-02, REV-03
**Success Criteria** (o que deve ser verdade):

  1. Em Repository access, o admin vê grupos, subgrupos e projetos (inclusive em namespaces aninhados) e escolhe quais revisar
  2. MRs abertos dos projetos selecionados aparecem na lista de revisões por polling; MRs em draft respeitam `skip_draft_prs`; MRs fechados ou mergeados saem da fila
  3. Um novo push num MR enfileira nova revisão e substitui a anterior
  4. A preparação busca `refs/merge-requests/<iid>/head` sem checkout, usa base, head e merge-base vivos, falha se o fetch falhar e calcula o diff localmente, sem truncar MRs grandes
  5. Um MR vindo de fork é preparado e revisado sem credenciais do fork, e escopo, retenção e URLs funcionam para `grupo/subgrupo/projeto`

**Plans**: 4 plans

Plans:
**Wave 1**

- [ ] 03-01-PLAN.md — GitLab read layer (glab api runner, projects, MRs, live state) + forge dispatch wrappers
- [ ] 03-02-PLAN.md — frontend: nested repoOwner, GitLab copy in Repository access

**Wave 2** *(blocked on 03-01)*

- [ ] 03-03-PLAN.md — git auth to GitLab, fail-closed gh guard, preparation skips, Scope 2+ segments

**Wave 3** *(blocked on 03-03)*

- [ ] 03-04-PLAN.md — server wiring: polling on, refresh via list_all, decoded namespace routes, auth status, release note
**UI hint**: yes

### Phase 4: Escrita GitLab: publicação dos resultados

**Goal**: Uma revisão validada aparece no MR como discussões inline, nota de status e resumo, sem duplicar nem postar em posições velhas
**Depends on**: Fase 3
**Requirements**: PUB-01, PUB-02, PUB-03, PUB-04, PUB-05, PUB-06, PUB-07
**Success Criteria** (o que deve ser verdade):

  1. Cada finding validado aparece como discussão inline na linha certa do diff do MR (adicionada, removida ou de contexto)
  2. Um finding cuja posição o GitLab rejeita aparece como nota geral com o mesmo marcador `<!-- ocr-... -->`, sem se perder
  3. A nota de status é criada uma vez e editada no lugar a cada etapa; o resumo é postado como nota, com a seção de sugestões quando Post suggestions está ligado
  4. Republicar ou repetir após falha parcial não cria comentários duplicados
  5. Se o MR recebeu push depois da revisão, a publicação é abortada; texto do modelo com `/merge`, `/approve`, menções ou auto-links não dispara nenhuma ação no GitLab

**Plans**: TBD

### Phase 5: Indexação, skills, UI e documentação

**Goal**: O deploy GitLab fica com paridade completa: indexação, skills, vocabulário e links do GitLab na UI, e documentação de configuração
**Depends on**: Fase 4
**Requirements**: IDX-01, IDX-02, UI-01, UI-02
**Success Criteria** (o que deve ser verdade):

  1. A indexação de um projeto GitLab conclui e mostra metadados, grafo de pacotes e símbolos como no GitHub
  2. Skills funcionam em projetos GitLab
  3. Num deploy GitLab a UI diz "merge request" e `!iid`, e os links abrem a instância GitLab
  4. Existe documentação de como configurar um deploy GitLab (host, CA, login) e uma release note em `changelog.d/`

**Plans**: TBD
**UI hint**: yes

## Tabela de progresso

| Fase | Planos concluídos | Status | Concluída em |
|------|-------------------|--------|--------------|
| 1. Camada de provider (Forge) | 1/1 | Complete   | 2026-11-05 |
| 2. Conexão com o GitLab | 5/5 | Complete   | 2026-11-05 |
| 3. Leitura GitLab | 0/4 | Planejada | - |
| 4. Escrita GitLab | 0/? | Não iniciada | - |
| 5. Indexação, UI e docs | 0/? | Não iniciada | - |
