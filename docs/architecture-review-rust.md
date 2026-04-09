# Architecture Review — Orbit Backend (Rust/Tauri)

> Gerado em: 2026-04-08
> Revisão aplicando [Apollo Rust Best Practices](https://github.com/apollographql/rust-best-practices)

---

## Diagnóstico Atual

Dois arquivos concentram lógica demais:

- **`commands.rs`** (500 linhas) — 8 domínios distintos misturados num único arquivo
- **`journal_reader.rs`** (700+ linhas) — dois caminhos de parsing separados (`parse_journal` e `process_line`) com ~250 linhas de lógica duplicada entre si

Resultado: adicionar um novo comando Tauri hoje exige navegar 500 linhas para encontrar onde inserir, e qualquer correção no parsing de mensagens precisa ser feita em dois lugares.

---

## Estrutura Proposta

```
tauri/src/
├── lib.rs                     # Só wiring (setup + invoke_handler)
├── models.rs                  # Tipos centrais (sem mudança)
│
├── ipc/                       # Handlers Tauri finos — recebe args, delega
│   ├── mod.rs
│   ├── session.rs             # (existente)
│   ├── project.rs             # (existente)
│   └── updater.rs             # (existente)
│
├── commands/                  # ← NOVO: lógica dos comandos Tauri
│   ├── mod.rs                 # re-exports
│   ├── diff.rs                # get_diff, get_file_versions
│   ├── files.rs               # list_project_files, get_subagent_journal
│   ├── tasks.rs               # get_tasks + parser de TodoWrite
│   ├── plugins.rs             # get_slash_commands, scan_plugin, frontmatter_field
│   └── stats.rs               # get_claude_usage_stats, get_changelog
│
├── journal/                   # ← SPLIT de journal_reader.rs (700 linhas → 3 arquivos)
│   ├── mod.rs
│   ├── state.rs               # JournalState, Default, context_percent()
│   ├── processor.rs           # process_line() — live streaming
│   └── parser.rs              # parse_journal() — file replay
│
├── services/
│   ├── database.rs            # (existente)
│   ├── session_manager.rs     # (existente, mais focado)
│   ├── spawn_manager.rs       # (existente)
│   └── worktree.rs            # (existente)
│
└── utils/
    ├── mod.rs
    ├── claude_paths.rs        # find_session_dir() — shared entre agent_tree e commands
    └── text.rs                # truncate_desc(), char_boundary()
```

### Como adicionar um novo comando após a refatoração

```rust
// 1. commands/meu_dominio.rs — criar função com lógica
#[tauri::command]
pub fn meu_novo_comando(arg: String) -> Result<Vec<MeuTipo>, String> {
    // faz uma coisa só
}

// 2. lib.rs — única mudança necessária
.invoke_handler(tauri::generate_handler![
    // ... existentes ...
    commands::meu_dominio::meu_novo_comando,  // ← adicionar aqui
])
```

---

## Tabela Completa de Melhorias

### Arquitetura / Modularização

| # | O quê | Arquivo atual | Destino | Impacto prático |
|---|-------|--------------|---------|-----------------|
| A1 | Split `commands.rs` em `commands/` | `commands.rs` (500 linhas, 8 domínios) | `commands/{diff,files,tasks,plugins,stats}.rs` | Adicionar comando novo = 1 função em 1 arquivo + 1 linha em `lib.rs`. Hoje exige navegar 500 linhas. |
| A2 | Split `journal_reader.rs` em `journal/` | `journal_reader.rs` (700+ linhas) | `journal/{state,processor,parser}.rs` | `process_line` e `parse_journal` em arquivos distintos — divergência entre os dois caminhos fica visível no diff. |
| A3 | Centralizar path do Claude em `utils/claude_paths.rs` | `agent_tree.rs:17` e `commands.rs:24` (duplicado) | `utils/claude_paths::find_session_dir()` | Um só lugar para mudar quando a estrutura de dirs do Claude mudar. |
| A4 | `utils/text.rs` com `truncate_desc` e `char_boundary` | `journal_reader.rs:413` e `commands.rs:86,117,145` (3 cópias) | `utils/text.rs` | Elimina a cópia divergente; `commands.rs` usa byte-slice hoje (bug real — veja C3). |
| A5 | `JournalEntry::default()` via `Default` trait | 8 call sites com 11 campos `None` cada | `impl Default for JournalEntry` | Adicionar novo campo em `JournalEntry` hoje quebra os 8 call sites. Com `Default`, só o campo novo precisa valor explícito. |
| A6 | `ipc/error.rs` com erro tipado via `thiserror` | `Result<T, String>` em todos os comandos | `Result<T, IpcError>` | Distinguir `SessionNotFound` de `DatabaseError` de `IoError` no frontend sem parsear string. |
| A7 | `SessionState::lock()` método helper | `.lock().unwrap()` em 15+ lugares | `impl SessionState { fn lock() }` | Poison recovery em um só lugar; todos os call sites ficam `state.lock()`. |
| A8 | `context_percent()` como método de `JournalState` | Calculado em `session_manager.rs` e `reader_loop` (2x) | `journal/state.rs::JournalState::context_percent()` | Fórmula e zero-guard em um lugar; mudança de modelo de tokens atualiza tudo. |

### Correção / Bugs

| # | O quê | Arquivo | Linha | Severidade | Descrição e fix |
|---|-------|---------|-------|-----------|-----------------|
| C1 | `std::mem::forget(child)` vaza handle do processo | `spawn_manager.rs` | 221 | **Crítico** | Unix: processo zumbi. Windows: kernel handle leak. Fix: mover `child` para dentro de `reader_loop` e deixar dropar no exit. |
| C2 | `delete_session` — dois DELETEs sem transaction | `database.rs` | 278–285 | **Alto** | Crash entre os dois DELETE = sessão sem outputs mas registro ainda na tabela. Fix: `conn.execute_batch("BEGIN/COMMIT")`. |
| C3 | Truncação com byte-slice em UTF-8 | `commands.rs` | 86, 117, 145 | **Médio** | `&desc[..77]` pânica se o caractere no índice 77 for multi-byte (`é`, `ñ`, emoji). Afeta `scan_plugin`. Fix: usar `char_boundary()` de `utils/text.rs`. |
| C4 | `days_to_date` reimplementa calendário | `commands.rs` | 453–494 | **Baixo** | `chrono` já é dependência. 40 linhas de loop manual com anos bissextos. Fix: `Utc.timestamp_opt(days as i64 * 86400, 0)`. |
| C5 | Mensagem de erro em português | `worktree.rs` | 45, 60 | **Baixo** | `"git não encontrado"` — inconsistente com convenção do projeto (inglês). |

### Performance / Velocidade

| # | O quê | Onde | Impacto | Fix |
|---|-------|------|---------|-----|
| P1 | `Vec::remove(0)` no `mini_log` — O(n) | `journal_reader.rs:272,742` | Shift de todos os elementos a cada tool call | `VecDeque` com `pop_front()` — O(1) |
| P2 | `to_lowercase()` aloca string por linha no hot path | `session_manager.rs:603` | Cada linha do stdout do Claude aloca uma `String` extra. Sessões longas: centenas de alocações desnecessárias | Busca case-insensitive sem alocar (memchr), ou mover check para stderr thread onde rate limits realmente aparecem |
| P3 | `restore_from_db` — replay síncrono de todas as sessões na startup | `session_manager.rs:556` | Bloqueia o thread principal. Com 50 sessões × 5k linhas = startup lenta | Lazy restore: rebuild `JournalState` só quando a sessão for acessada pela primeira vez |
| P4 | `insert_output` — 1 INSERT por linha do Claude | `database.rs:239` | Claude rápido: 50–100 linhas/seg = 50–100 INSERTs/seg + Mutex lock por linha | Canal interno com batching: acumular linhas e fazer INSERT em lote a cada 100ms |
| P5 | `Mutex<Connection>` — todas as ops do DB serializam | `database.rs:8` | Leitura de journal e escrita de output competem pelo mesmo lock | WAL mode: `PRAGMA journal_mode=WAL` — 1 linha, libera leituras concorrentes |
| P6 | `derive_status_from_tail` reabre o arquivo | `journal_reader.rs:466` | Chamado no final de `parse_journal`, abre o mesmo arquivo pela segunda vez só para ler o tail | Passar o `BufReader` já aberto ao invés de reler |
| P7 | `get_sessions` full-table query + merge a cada chamada | `session_manager.rs:489` | Lista completa relida do DB em toda chamada do frontend | Cachear snapshot de `Vec<Session>`, invalidar só em mutações (`create`, `stop`, `delete`) |
| P8 | `Mutex` para tudo — inclui leituras de `journal_states` | `session_manager.rs` | `journal_states` e `active` são lidos muito mais do que escritos; Mutex bloqueia leitores entre si | `RwLock<SessionManager>` ou separar `journal_states` em `RwLock` próprio |

### Concorrência / Segurança de Thread

| # | O quê | Impacto |
|---|-------|---------|
| T1 | `.lock().unwrap()` em 15+ lugares — panic em Mutex envenenado | Se qualquer thread panicar segurando o lock, o Mutex fica envenenado e **todas** as chamadas subsequentes propagam panic. Fix: `unwrap_or_else(\|e\| e.into_inner())` centralizado no método `SessionState::lock()`. |
| T2 | `session:rate-limit` emitido duas vezes para a mesma sessão | `reader_loop` (stdout JSON) e stderr thread detectam rate limit independentemente. Frontend pode receber o evento duplicado. Fix: flag `rate_limit_emitted: AtomicBool` ou deduplicar no frontend. |
| T3 | `do_spawn` trava o Mutex 3 vezes separadas na mesma lógica | Lines 251–256, 287–291, 301 — cada lock/unlock abre janela para outra thread modificar o estado entre as operações. Fix: consolidar em menos pontos de lock ou usar escopo único. |

### Manutenibilidade / Testes

| # | O quê | Impacto |
|---|-------|---------|
| M1 | `parse_journal` e `process_line` têm lógica duplicada (~250 linhas) | Bug no parsing de tool results precisa ser corrigido nos dois caminhos. `parse_journal` deveria chamar `process_line` linha a linha. |
| M2 | `migrate()` usa `ALTER TABLE` com erros ignorados | Migrations não-idempotentes falham silenciosamente. Usar tabela `_migrations` (já existe no schema) para rastrear o que foi aplicado. |
| M3 | Zero testes para `parse_journal` | Session replay pode divergir de live-stream sem cobertura detectando. |
| M4 | Zero testes para `get_tasks`, `scan_plugin`, `get_claude_usage_stats` | Os três comandos mais complexos de `commands.rs` sem nenhum teste. |
| M5 | `Session.status` é `String` mas enum `SessionStatus` existe | `assert_eq!(s.status, "stopped")` — typo passa silenciosamente. Fix: enum no campo + serde serializa como string. |

---

## Prioridade de Implementação

```
🔴 Fazer primeiro (correctness + maior ROI arquitetural)
   C1 — mem::forget child (bug de OS)
   C2 — DELETE não-atômico (corrupção de dados)
   A1 — commands/ folder (desbloqueador de produtividade)
   A2 — journal/ split (elimina duplicação crítica M1)

🟡 Próxima iteração (performance + manutenibilidade)
   P3 — restore_from_db lazy (startup)
   P5 — WAL mode (1 linha, ganho imediato de concorrência)
   P4 — batching de INSERT (throughput em sessões ativas)
   A7 — SessionState::lock() (segurança de thread)
   M1 — parse_journal chamar process_line

🟢 Refinamento (qualidade + ergonomia)
   T1 — Mutex poison recovery
   A5 — JournalEntry::default()
   A6 — thiserror IpcError
   C3 — UTF-8 truncation fix
   P2 — rate limit sem alocação
   M5 — Session.status como enum
   P8 — RwLock para journal_states
```

---

## Exemplos de Código

### A7 — SessionState::lock()

```rust
impl SessionState {
    pub fn lock(&self) -> std::sync::MutexGuard<'_, SessionManager> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

// Antes: state.0.lock().unwrap()
// Depois: state.lock()
```

### A8 — context_percent() como método

```rust
impl JournalState {
    pub fn context_percent(&self) -> f64 {
        let window = self.model.as_deref()
            .map(crate::models::context_window)
            .unwrap_or(200_000);
        if window == 0 { return 0.0; }
        ((self.input_tokens + self.output_tokens) as f64 / window as f64) * 100.0
    }
}
```

### P5 — WAL mode (1 linha)

```rust
fn migrate(&self) -> SqlResult<()> {
    let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;  // ← adicionar aqui
    // ... resto das migrations
}
```

### C2 — delete_session atômico

```rust
pub fn delete_session(&self, id: SessionId) -> SqlResult<()> {
    let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
    conn.execute_batch("BEGIN")?;
    conn.execute("DELETE FROM session_outputs WHERE session_id = ?1", params![id])?;
    conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
    conn.execute_batch("COMMIT")?;
    Ok(())
}
```

### A3 — claude_paths helper

```rust
// utils/claude_paths.rs
pub fn find_session_dir(session_id: &str) -> Option<std::path::PathBuf> {
    let projects_dir = dirs::home_dir()?.join(".claude").join("projects");
    std::fs::read_dir(projects_dir).ok()?
        .flatten()
        .map(|e| e.path().join(session_id))
        .find(|p| p.is_dir())
}
```
