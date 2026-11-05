# Requirements: OpenCodeReview — GitLab self-hosted

**Defined:** 2026-11-05
**Core Value:** Um MR aberto numa instância GitLab self-hosted (com CA interna) é revisado e recebe findings inline automaticamente, sem quebrar nada do fluxo GitHub existente.

## v1 Requirements

Escopo da v1: dar acesso ao GitLab com paridade exata ao fluxo GitHub atual. Nada além do que o GitHub já faz.

### Fundação (FORGE)

- [x] **FORGE-01**: Admin escolhe o provider do deploy (GitHub ou GitLab) por configuração; o server se recusa a iniciar se o provider ou o host mudar sobre dados existentes
- [x] **FORGE-02**: Fluxo GitHub continua idêntico após a camada de provider (testes, smokes, corpos de comentário e URLs)
- [ ] **FORGE-03**: Projetos em namespaces aninhados (grupo/subgrupo/projeto) funcionam em todo o produto (escopo, retenção, URLs)

### Conexão (AUTH)

- [ ] **AUTH-01**: Admin conecta o GitLab colando um PAT em Settings; o token vai ao `glab` via stdin e nunca aparece no browser, logs ou banco
- [ ] **AUTH-02**: Settings mostra o status da conexão GitLab (host, usuário) e um erro acionável quando o token não tem escopo `api`, está expirado ou o host não responde
- [ ] **AUTH-03**: Admin pode desconectar o GitLab
- [x] **AUTH-04**: O server confia na CA interna montada no container para `glab` e `git`, sem opção de desligar a verificação TLS
- [ ] **AUTH-05**: `glab` é instalado só na imagem do server, com versão fixa e checksum verificado; runners não recebem `glab`, token nem CA

### Repositórios e descoberta (REPO)

- [ ] **REPO-01**: Admin vê grupos, subgrupos e projetos do GitLab em Repository access e escolhe quais revisar
- [ ] **REPO-02**: MRs abertos dos projetos selecionados são descobertos por polling e aparecem na lista de revisões
- [ ] **REPO-03**: Um novo push no MR enfileira nova revisão e substitui a anterior
- [ ] **REPO-04**: MRs em draft seguem a opção `skip_draft_prs` por repositório, como no GitHub
- [ ] **REPO-05**: MRs fechados ou mergeados saem da fila

### Preparação da revisão (REV)

- [ ] **REV-01**: A revisão de um MR usa base, head e merge-base vivos via fetch de `refs/merge-requests/<iid>/head`, sem checkout; fetch falho falha a preparação
- [ ] **REV-02**: MRs vindos de fork são revisados sem credenciais do fork
- [ ] **REV-03**: O diff é calculado localmente, então MRs grandes não são truncados pelos limites da API

### Publicação (PUB)

- [ ] **PUB-01**: Findings validados são postados como discussões inline no diff do MR, na linha correta (adicionada, removida ou de contexto)
- [ ] **PUB-02**: Finding cuja posição o GitLab rejeita vira nota geral com o mesmo marcador, sem perder o finding
- [ ] **PUB-03**: A nota de status no MR é criada uma vez e editada no lugar a cada etapa
- [ ] **PUB-04**: O resumo da revisão é postado como nota, com a seção de sugestões quando Post suggestions está ligado
- [ ] **PUB-05**: Republicar ou repetir após falha parcial não duplica comentários (marcadores `<!-- ocr-... -->`)
- [ ] **PUB-06**: Se o MR recebeu push depois da revisão, a publicação é abortada em vez de postar em posições desatualizadas
- [ ] **PUB-07**: Texto do modelo não dispara quick actions (`/merge`, `/approve`), menções ou auto-links no GitLab

### Indexação (IDX)

- [ ] **IDX-01**: A indexação de repositórios funciona para projetos GitLab
- [ ] **IDX-02**: Skills funcionam para projetos GitLab

### Interface e docs (UI)

- [ ] **UI-01**: Num deploy GitLab, a UI usa termos do GitLab (merge request, `!iid`) e links para a instância
- [ ] **UI-02**: Há documentação de como configurar um deploy GitLab (host, CA, login) e uma release note em `changelog.d/`

## v2 Requirements

Adiados. Registrados, mas fora do roadmap atual.

### Publicação

- **PUB-V2-01**: Findings que cobrem um intervalo de linhas usam comentários multi-linha (`line_range`)
- **PUB-V2-02**: Sugestões aplicáveis inline (blocos ```` ```suggestion ````) — exige mudar o invariante que lista sugestões só no resumo
- **PUB-V2-03**: Discussões do bot são resolvidas quando um novo push corrige o finding

### Outros

- **MISC-V2-01**: Labels no MR conforme o resultado da revisão
- **MISC-V2-02**: PR upstream para a BridgLabs

## Out of Scope

| Feature | Reason |
|---------|--------|
| GitHub e GitLab no mesmo deploy | Decisão do usuário: um provider por deploy |
| Várias instâncias GitLab por deploy | Uma instância por deploy, como `OCR_GH_HOST` |
| Webhooks do GitLab | O produto descobre PRs por polling; mantém o modelo |
| Tokens gerenciados pelo app | Autenticação delegada ao `glab`, espelhando o `gh` |
| Aprovar, mergear ou dar push no MR | Invariante do produto: nada aprova automaticamente |
| Benchmark (`ocr-bench`) para GitLab | Benchmark independe de hospedagem |
| Comentários multi-linha e sugestões inline na v1 | Além da paridade com o fluxo GitHub atual |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| AUTH-01 | Fase 2 | Pendente |
| AUTH-02 | Fase 2 | Pendente |
| AUTH-03 | Fase 2 | Pendente |
| AUTH-04 | Fase 2 | Pendente |
| AUTH-05 | Fase 2 | Pendente |
| FORGE-01 | Fase 2 | Pendente |
| FORGE-02 | Fase 1 | Pendente |
| FORGE-03 | Fase 3 | Pendente |
| IDX-01 | Fase 5 | Pendente |
| IDX-02 | Fase 5 | Pendente |
| PUB-01 | Fase 4 | Pendente |
| PUB-02 | Fase 4 | Pendente |
| PUB-03 | Fase 4 | Pendente |
| PUB-04 | Fase 4 | Pendente |
| PUB-05 | Fase 4 | Pendente |
| PUB-06 | Fase 4 | Pendente |
| PUB-07 | Fase 4 | Pendente |
| REPO-01 | Fase 3 | Pendente |
| REPO-02 | Fase 3 | Pendente |
| REPO-03 | Fase 3 | Pendente |
| REPO-04 | Fase 3 | Pendente |
| REPO-05 | Fase 3 | Pendente |
| REV-01 | Fase 3 | Pendente |
| REV-02 | Fase 3 | Pendente |
| REV-03 | Fase 3 | Pendente |
| UI-01 | Fase 5 | Pendente |
| UI-02 | Fase 5 | Pendente |

**Coverage:**
- v1 requirements: 27 total
- Mapped to phases: 27
- Unmapped: 0 ✓

---
*Requirements defined: 2026-11-05*
*Last updated: 2026-11-05 after initial definition*
