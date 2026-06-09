# Daemon → Feed — estratégia de testes

Como o caminho "evento do SDK → chat" é testado no orbit, e por quê cada forma
foi escolhida. Coverage atual dos módulos `ui/lib/daemon-*.ts`: **~99% stmts,
100% funcs**.

## Camadas e o teste certo para cada uma

| Módulo | O que é | Forma de teste | Arquivo |
|--------|---------|----------------|---------|
| `daemon-feed.ts` | Mapper puro `DaemonEvent → JournalEntry` + classificação | **Homologação + parametrizado por provider** | `daemon-feed.test.ts` |
| `daemon-runs.ts` | Registry `runId↔sessionId` | Unidade, estado isolado por `resetDaemonRuns()` | `daemon-runs.test.ts` |
| `daemon-api.ts` | Client REST do daemon | Unidade com **`fetch` injetado** (sem rede/globals) | `daemon-api.test.ts` |
| `daemon-session.ts` | Orchestrators (compõem api+registry) | Unidade, `fetch` injetado | `daemon-session.test.ts` |
| `daemon-client.ts` | Bridge SSE → journal | Unidade (acumulação) + **integração** | `daemon-client.test.ts`, `daemon-pipe.integration.test.ts` |

## Princípios aplicados

1. **Homologação como contrato, não exemplo.** O teste enumera o catálogo
   completo de eventos do SDK e falha se algum tipo ficar sem classificação
   `render|ignore`. Um tipo novo no daemon quebra o teste — força decisão
   consciente, em vez de o evento sumir do chat silenciosamente.

2. **Funções puras no núcleo.** `mapDaemonEvent` não toca store, rede nem tempo.
   Recebe `{sessionId, seq}` por parâmetro. Testar é só asserção sobre o retorno
   — rápido, determinístico, sem mocks.

3. **Injeção de dependência nas bordas.** `fetch` é parâmetro (`FetchLike`),
   não global. Zero `vi.mock` de módulo; o teste passa um stub e inspeciona a
   chamada. Mais legível e à prova de refactor de import.

4. **Parametrização por provider.** Onde o comportamento deve ser igual entre
   claude/codex/opencode, o teste roda os três (`it.each`) e compara, em vez de
   confiar num provider só. Garante o "independente do provedor".

5. **Um teste de integração que cruza tudo.** `daemon-pipe.integration.test.ts`
   usa um **fake `EventSource`** para empurrar frames SSE reais por
   `connectDaemonSse → bridge → mapper → registry → journal`. É o teste que prova
   a promessa de ponta: um evento emitido cai no feed da sessão certa. Testar as
   peças isoladas não pega erro de fiação (ex.: resolver recebendo o event em vez
   do runId — bug real pego por este teste).

6. **Estado global resetado em `beforeEach`.** Registry e `journal` são
   singletons; cada teste limpa para não vazar estado.

## Lacunas conhecidas / não cobertas de propósito

- `entryTypeFor` tem um `return null` defensivo inalcançável hoje (todo tipo
  `render` mapeia). Mantido como guarda; não vale teste artificial.
- Branches de borda em `mergeStreamed` (output/exitCode) ficam ~86% de branch —
  caminhos defensivos, baixo risco.

## Próximo nível (quando integrar de verdade)

- **Contract test com servidor real.** Subir o daemon Fastify em memória
  (`app.inject` / supertest), abrir um run e consumir `/events` de ponta a ponta.
  Pega divergências de schema entre os dois repos que o mirror de tipos não pega.
- **Sincronizar o catálogo automaticamente.** Hoje `SDK_EVENT_CATALOG` é mantido
  à mão (com o comando `grep` documentado no teste). Um script que extrai os
  tipos do `daemon/src` e compara com `EVENT_CLASSIFICATION` no CI elimina drift.
- **Snapshot de render por tipo.** Um component test que monta o `Feed` com um
  exemplo de cada `entryType` e snapshota — pega regressão visual de homologação.
