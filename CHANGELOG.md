# Changelog

Todas as alterações significativas do projeto Orbit são registradas aqui.
Gerado automaticamente pelo hook `commit-msg` a cada commit.

---

## [2026-04-06 15:36] feat: bash auto-run, progress streaming e rate limit handling
**Branch:** `master` · **Autor:** josefernando · 10 files changed, 235 insertions(+), 20 deletions(-)

### Detalhes
- Remove pending_approval para Bash (auto-run via --dangerously-skip-permissions)
- Status do agente permanece Working durante execução de Bash (não Input)
- Captura eventos progress do Claude para exibir output em tempo real
- Novo tipo Progress em JournalEntryType; Feed.svelte agrupa com toolCall
- Captura stderr do processo Claude para detectar rate limit
- Emite session:rate-limit via Tauri; banner no frontend com auto-dismiss de 30s
- kill_pid via taskkill no Windows ao parar sessão

### Arquivos alterados
  - api/App.svelte
  - api/components/Feed.svelte
  - api/components/MetaPanel.svelte
  - api/components/ToolCallEntry.svelte
  - api/lib/tauri.ts
  - api/lib/types.ts
  - front/src/journal_reader.rs
  - front/src/models.rs
  - front/src/services/session_manager.rs
  - front/src/services/spawn_manager.rs

---

## [2026-04-06] refactor: remove unnecessary comments across TS, Svelte, and Rust sources
**Branch:** `master` · **Autor:** josefernando

### Arquivos alterados
  - (múltiplos arquivos — limpeza geral de comentários desnecessários)

---
