# OpenRouter Provider — Spec & Progress

## Objetivo

Adicionar OpenRouter como provider alternativo ao Claude Code CLI no Orbit, permitindo usar qualquer modelo disponível no OpenRouter (centenas de modelos via API OpenAI-compatible).

## Comportamento Esperado

- Ao criar sessão, usuário escolhe provider: **Claude Code** (padrão) ou **OpenRouter**
- OpenRouter requer: API key + nome do modelo (ex: `anthropic/claude-sonnet-4`)
- Sessão OpenRouter usa HTTP SSE streaming (não subprocess)
- Feed, tokens, context % funcionam igual para ambos providers
- Follow-up messages enviam histórico completo (OpenRouter é stateless)
- Sessões de providers diferentes coexistem no mesmo app

## Arquitetura

```
NewSessionModal → provider selector
                    ↓
              create_session(provider: "openrouter")
                    ↓
              session_manager.do_spawn()
                    ↓ dispatch by provider
        ┌───────────┴───────────┐
   claude-code              openrouter
   spawn_claude()          spawn_openrouter()
   subprocess              HTTP SSE → pipe
        └───────────┬───────────┘
              reader_loop() ← Box<dyn Read + Send>
                    ↓
              process_line() → JournalEntry
                    ↓
              session:output event → Feed
```

**Key insight:** Ambos providers produzem `Box<dyn Read + Send>` com JSONL lines → `reader_loop()` e `process_line()` são reutilizados sem mudanças.

## Mudanças no DB

```sql
ALTER TABLE sessions ADD COLUMN provider TEXT DEFAULT 'claude-code';
```

## API Key

- Armazenada APENAS em memória (`ActiveSession.api_key`)
- Nunca persistida no DB (segurança)
- Usuário insere ao criar sessão OpenRouter
- Perde ao fechar app (re-insere ao criar nova sessão)

## Modelos OpenRouter

- Input livre (text field) — centenas de modelos disponíveis
- Exemplos: `anthropic/claude-sonnet-4`, `meta-llama/llama-3.1-70b`, `google/gemini-2.0-flash`
- Futuro: query `GET /v1/models` para autocomplete

## Casos de Borda

- API key inválida → sessão entra em status Error com mensagem clara
- Modelo inexistente → erro do OpenRouter, exibido no feed
- Rate limit OpenRouter → detectado no response, exibido como toast
- App restart → sessão OpenRouter perde API key, precisa recriar
- `/effort` não se aplica a OpenRouter → comando desabilitado
- `/model` funciona (muda modelo para próxima mensagem)
- Worktree não se aplica a OpenRouter → toggle desabilitado

## Critérios de Aceitação

- [ ] Criar sessão OpenRouter com API key e modelo
- [ ] Resposta streama no feed em tempo real
- [ ] Tokens e context % atualizam corretamente
- [ ] Follow-up messages funcionam (histórico enviado)
- [ ] Sessão Claude Code continua funcionando normalmente ao lado
- [ ] Provider exibido na sidebar/MetaPanel
- [ ] Comandos Claude-specific desabilitados para OpenRouter

---

## Plano de Implementação

Plano completo em: `docs/superpowers/plans/2026-04-12-openrouter-provider.md`

### Progress

| # | Task | Status | Commit |
|---|------|--------|--------|
| 1 | DB migration — add `provider` column | ✅ Done | `62a92be` |
| 2 | Rust models — add `provider` to Session struct | 🔄 In Progress | `provider` field added to struct, callers pending |
| 3 | IPC layer — accept `provider` in create_session | ⏳ Pending | blocked by #2 |
| 4 | Frontend types — add `provider` to TS interfaces | ⏳ Pending | blocked by #3 |
| 5 | NewSessionModal — provider selector UI | ⏳ Pending | blocked by #4 |
| 6 | Add `reqwest` dependency | ⏳ Pending | — |
| 7 | spawn_openrouter — HTTP SSE to JSONL reader | ⏳ Pending | blocked by #6 |
| 8 | Session manager — provider dispatch in do_spawn | ⏳ Pending | blocked by #3, #7 |
| 9 | InputBar — provider-aware commands | ⏳ Pending | blocked by #4 |
| 10 | API key management — secure in-memory storage | ⏳ Pending | blocked by #3 |
| 11 | Integration test — end-to-end | ⏳ Pending | blocked by #5, #8, #9, #10 |

### Dependency Graph

```
Task 1 → Task 2 → Task 3 → Task 4 → Task 5
                       ↓         ↓       ↓
                    Task 10   Task 9   Task 11
                       ↓                  ↑
Task 6 → Task 7 → Task 8 ────────────────┘
```

### Notas

- Task 2 parcialmente feita: campo `provider: String` adicionado a `Session` em `models.rs`, falta atualizar callers (session_manager, database queries, tests)
- `rate_limit_event` aparece no stream `-p` do Claude — pode ser usado para rate limits no futuro
- OpenRouter API é OpenAI-compatible, stream format similar ao Claude JSONL
