# Padrões de Testes Unitários — Orbit Backend (Rust)

> Última atualização: 2026-04-08

---

## TestCase — Helper de Log

Todo teste usa `crate::test_utils::TestCase`. Ele imprime um header formatado no início, marca cada assertion com `✓` ou `✗`, e exibe `PASSED / FAILED` ao final.

```
┌──────── should_create_project_with_correct_name ────────┐
  ▸ Seed
  ▸ Act
  ▸ Assert
  ✓ project.name
  ✓ project.id is positive
  └─ PASSED (2 checks)
```

**Para ver o output:** `cargo test -- --nocapture`  
**Um teste que falha** mostra o output automaticamente (cargo exibe stdout de testes que falham).

### API do TestCase

```rust
use crate::test_utils::TestCase;

let mut t = TestCase::new("nome_do_teste");  // imprime o header

t.phase("Seed");    // label de fase — não conta como assertion
t.phase("Act");
t.phase("Assert");

t.ok("desc", condition: bool);              // condition == true
t.eq("desc", left: T, right: T);           // left == right, imprime ambos se falhar
t.ne("desc", left: T, right: T);           // left != right
t.some("desc", opt: &Option<T>);           // is_some()
t.none("desc", opt: &Option<T>);           // is_none(), imprime valor se falhar
t.is_ok("desc", result: &Result<T, E>);   // is_ok(), imprime Err se falhar
t.is_err("desc", result: &Result<T, E>);  // is_err(), imprime Ok se falhar
t.len("desc", slice: &[T], n: usize);     // len() == n
t.empty("desc", slice: &[T]);             // len() == 0
```

---

## Princípios Gerais

1. **Banco real, sem mocks** — testes de integração usam `DatabaseService::open_in_memory()`. Nunca mockar o banco de dados; divergência mock/produção já causou falhas silenciosas neste projeto.
2. **Isolamento total** — cada teste cria sua própria instância de DB ou arquivo temporário. Nenhum teste depende do estado de outro.
3. **Padrão Seed → Act → Assert → Cleanup** — toda sequência de teste segue este ciclo explicitamente, marcado com `t.phase(...)`.
4. **Uma asserção por comportamento** — prefira um `t.eq` / `t.ok` por teste. Se precisar verificar vários aspectos, quebre em testes separados.
5. **Nomes descritivos** — nomes no formato `should_<comportamento>_when_<condição>`.
6. **Testes devem documentar comportamento** — ao ler o teste, deve ser possível entender o que o sistema faz sem olhar a implementação.

---

## Regras Críticas

| Regra | Por quê |
|-------|---------|
| Nunca use `unwrap()` em setup de teste sem contexto | Falha silenciosa de setup mascara o teste real. Use `expect("mensagem clara")`. |
| Nunca compartilhe estado entre testes (`static`, `lazy_static`, `OnceLock`) | Ordem de execução de testes não é garantida. |
| Nunca teste múltiplos comportamentos independentes em um teste | Falha na primeira asserção oculta as demais. |
| Sempre use `tempfile::TempDir` para arquivos em testes | Garante cleanup automático mesmo se o teste panicar. |
| Nunca faça `thread::sleep` em testes | Torna a suite lenta e flaky. Use canais ou primitivas de sincronização. |

---

## Estrutura de Arquivo de Teste

Cada módulo mantém seus testes no próprio arquivo, no bloco `#[cfg(test)]` ao final:

```rust
// src/services/database.rs

pub struct DatabaseService { /* ... */ }

impl DatabaseService {
    pub fn alguma_funcao(&self) { /* ... */ }
}

// ─── Testes ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestCase;

    // ── Helpers de setup ────────────────────────────────────────────────────

    fn make_db() -> DatabaseService {
        DatabaseService::open_in_memory()
            .expect("test setup: failed to open in-memory DB")
    }

    fn seed_session(db: &DatabaseService) -> crate::models::SessionId {
        db.create_session(None, Some("test-session"), "/tmp", "ignore", None)
            .expect("test setup: failed to seed session")
    }

    // ── Testes ──────────────────────────────────────────────────────────────

    #[test]
    fn should_create_project_with_correct_name() {
        let mut t = TestCase::new("should_create_project_with_correct_name");

        t.phase("Seed");
        let db = make_db();

        t.phase("Act");
        let project = db.create_project("my-app", "/home/user/my-app")
            .expect("create_project failed");

        t.phase("Assert");
        t.eq("project.name", &project.name, &"my-app".to_string());
        t.ok("project.id is positive", project.id > 0);

        // Cleanup: DB dropped automaticamente
    }

    #[test]
    fn should_return_existing_project_when_path_already_exists() {
        let mut t = TestCase::new("should_return_existing_project_when_path_already_exists");

        t.phase("Seed");
        let db = make_db();
        let first = db.create_project("my-app", "/home/user/my-app")
            .expect("first create failed");

        t.phase("Act");
        let second = db.create_project("my-app", "/home/user/my-app")
            .expect("second create failed");

        t.phase("Assert");
        t.eq("same ID returned (idempotente)", first.id, second.id);
    }
}
```

---

## Padrão Seed → Act → Assert → Cleanup

### DB In-Memory (caso mais comum)

O cleanup é **automático**: o `DatabaseService` é dropado ao final do teste, junto com o banco em memória.

```rust
#[test]
fn should_list_sessions_ordered_by_created_at_desc() {
    let mut t = TestCase::new("should_list_sessions_ordered_by_created_at_desc");

    // ── Seed ──────────────────────────────────────────────────────────────
    t.phase("Seed");
    let db = make_db();
    let id1 = db.create_session(None, Some("first"), "/a", "ignore", None)
        .expect("seed session 1");
    let id2 = db.create_session(None, Some("second"), "/b", "ignore", None)
        .expect("seed session 2");

    // ── Act ───────────────────────────────────────────────────────────────
    t.phase("Act");
    let sessions = db.get_sessions().expect("get_sessions failed");

    // ── Assert ────────────────────────────────────────────────────────────
    t.phase("Assert");
    t.eq("mais recente vem primeiro", sessions[0].id, id2);
    t.eq("primeiro criado vem segundo", sessions[1].id, id1);

    // ── Cleanup ───────────────────────────────────────────────────────────
    // (automático — db dropado aqui)
}
```

### Arquivo Temporário (parse_journal, diff, etc.)

Use `tempfile::TempDir` para cleanup automático mesmo em caso de panic:

```rust
use tempfile::TempDir;
use std::io::Write;

fn write_jsonl(dir: &TempDir, filename: &str, lines: &[&str]) -> std::path::PathBuf {
    let path = dir.path().join(filename);
    let mut f = std::fs::File::create(&path).expect("failed to create temp file");
    for line in lines {
        writeln!(f, "{}", line).expect("failed to write line");
    }
    path
}

#[test]
fn should_parse_assistant_text_from_jsonl_file() {
    let mut t = TestCase::new("should_parse_assistant_text_from_jsonl_file");

    // ── Seed ──────────────────────────────────────────────────────────────
    t.phase("Seed");
    let dir = TempDir::new().expect("failed to create temp dir");
    let path = write_jsonl(&dir, "session.jsonl", &[
        r#"{"type":"assistant","timestamp":"2026-01-01T00:00:00Z","message":{"model":"claude-sonnet-4-6","content":[{"type":"text","text":"Hello!"}],"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#,
    ]);

    // ── Act ───────────────────────────────────────────────────────────────
    t.phase("Act");
    let state = crate::journal_reader::parse_journal(&path, 0, None);

    // ── Assert ────────────────────────────────────────────────────────────
    t.phase("Assert");
    t.len("one entry parsed", &state.entries, 1);
    t.eq("entry text is Hello!", state.entries[0].text.as_deref(), Some("Hello!"));

    // ── Cleanup ───────────────────────────────────────────────────────────
    // (automático — TempDir dropado aqui, arquivo deletado)
}
```

### Múltiplos Ciclos no Mesmo Teste

Quando o comportamento exige múltiplos estados sequenciais (ex: criar → atualizar → deletar), faça ciclos explícitos comentados:

```rust
#[test]
fn should_remove_session_from_active_and_journal_after_delete() {
    let mut t = TestCase::new("should_remove_session_from_active_and_journal_after_delete");

    // ── Ciclo 1: Seed ─────────────────────────────────────────────────────
    t.phase("Seed");
    let db = Arc::new(make_db());
    let mut sm = SessionManager::new(Arc::clone(&db));
    let session = sm.init_session("/tmp/proj", Some("feat"), "ignore", None, false)
        .expect("init_session failed");

    // ── Ciclo 1: Assert (session ativa) ───────────────────────────────────
    t.phase("Assert — session ativa");
    t.ok("session está ativa após init", sm.is_session_active(session.id));

    // ── Ciclo 2: Act (deletar) ────────────────────────────────────────────
    t.phase("Act — deletar session");
    sm.delete_session(session.id).expect("delete failed");

    // ── Ciclo 2: Assert (session removida) ────────────────────────────────
    t.phase("Assert — session removida");
    t.ok("não está mais no active map", !sm.is_session_active(session.id));
    t.ok("journal_state removido", !sm.journal_states.contains_key(&session.id));
    t.empty("sem sessões no DB", &db.get_sessions().unwrap());

    // ── Cleanup ───────────────────────────────────────────────────────────
    // (automático)
}
```

---

## Helpers de Seed

Centralize helpers de seed no topo do bloco `mod tests` de cada arquivo. Isso evita repetição e facilita mudanças de schema.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    // ── Helpers de Setup ──────────────────────────────────────────────────

    /// Cria um DatabaseService em memória pronto para uso.
    fn make_db() -> Arc<DatabaseService> {
        Arc::new(DatabaseService::open_in_memory()
            .expect("failed to open in-memory DB"))
    }

    /// Cria um SessionManager conectado a um DB em memória.
    fn make_manager() -> Arc<Mutex<SessionManager>> {
        Arc::new(Mutex::new(SessionManager::new(make_db())))
    }

    /// Insere uma sessão e retorna o ID. Panics com mensagem clara se falhar.
    fn seed_session(db: &DatabaseService) -> crate::models::SessionId {
        db.create_session(None, Some("test"), "/tmp/proj", "ignore", None)
            .expect("seed_session: create_session failed")
    }

    /// Insere N linhas de output em uma sessão.
    fn seed_outputs(db: &DatabaseService, session_id: crate::models::SessionId, lines: &[&str]) {
        for line in lines {
            db.insert_output(session_id, line)
                .expect("seed_outputs: insert_output failed");
        }
    }

    /// Linha JSONL de assistant válida para testes.
    fn assistant_line(text: &str) -> String {
        format!(
            r#"{{"type":"assistant","timestamp":"2026-01-01T00:00:00Z","message":{{"model":"claude-sonnet-4-6","content":[{{"type":"text","text":"{text}"}}],"usage":{{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}}}}"#
        )
    }

    /// Linha JSONL de tool_use válida para testes.
    fn tool_use_line(tool: &str, file_path: &str) -> String {
        format!(
            r#"{{"type":"assistant","timestamp":"2026-01-01T00:00:00Z","message":{{"model":"claude-sonnet-4-6","content":[{{"type":"tool_use","name":"{tool}","id":"toolu_01","input":{{"file_path":"{file_path}"}}}}],"usage":{{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}}}}"#
        )
    }

    /// Linha JSONL de tool_result para fechar um tool_use.
    fn tool_result_line(content: &str) -> String {
        format!(
            r#"{{"type":"user","timestamp":"2026-01-01T00:00:01Z","message":{{"content":[{{"type":"tool_result","tool_use_id":"toolu_01","content":"{content}"}}]}}}}"#
        )
    }
}
```

---

## Cobertura Obrigatória por Módulo

### `services/database.rs` ✅ (coberto)

Adicionar os testes ausentes:

```rust
#[test]
fn should_delete_session_and_outputs_atomically() {
    // Seed
    let db = make_db();
    let id = seed_session(&db);
    seed_outputs(&db, id, &[r#"{"type":"assistant"}"#, r#"{"type":"user"}"#]);

    // Act
    db.delete_session(id).expect("delete failed");

    // Assert — sessão e outputs removidos
    assert!(db.get_session(id).unwrap().is_none());
    assert_eq!(db.get_outputs(id).unwrap().len(), 0);
}

#[test]
fn should_persist_claude_session_id_and_retrieve_it() {
    // Seed
    let db = make_db();
    let id = seed_session(&db);

    // Act
    db.update_claude_session_id(id, "claude-abc-123").expect("update failed");
    let result = db.get_claude_session_id(id).expect("get failed");

    // Assert
    assert_eq!(result, Some("claude-abc-123".to_string()));
}
```

### `journal_reader.rs` — `process_line` ✅ / `parse_journal` ❌

Testes faltantes para `parse_journal`:

```rust
#[test]
fn should_parse_thinking_block_from_file() {
    // Seed
    let dir = TempDir::new().unwrap();
    let path = write_jsonl(&dir, "s.jsonl", &[
        r#"{"type":"assistant","timestamp":"2026-01-01T00:00:00Z","message":{"model":"claude-sonnet-4-6","content":[{"type":"thinking","thinking":"Let me think..."}],"usage":{"input_tokens":5,"output_tokens":2,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#,
    ]);

    // Act
    let state = parse_journal(&path, 0, None);

    // Assert
    assert_eq!(state.entries.len(), 1);
    assert_eq!(state.entries[0].entry_type, JournalEntryType::Thinking);
    assert_eq!(state.entries[0].thinking.as_deref(), Some("Let me think..."));
}

#[test]
fn should_parse_tool_use_and_result_from_file() {
    // Seed
    let dir = TempDir::new().unwrap();
    let path = write_jsonl(&dir, "s.jsonl", &[
        &tool_use_line("Read", "/src/main.rs"),
        &tool_result_line("file content here"),
    ]);

    // Act
    let state = parse_journal(&path, 0, None);

    // Assert
    assert_eq!(state.entries.len(), 2);
    assert_eq!(state.entries[0].entry_type, JournalEntryType::ToolCall);
    assert_eq!(state.entries[1].entry_type, JournalEntryType::ToolResult);
}

#[test]
fn should_produce_same_entries_as_process_line_for_identical_input() {
    // Seed
    let lines = [
        assistant_line("Hello!"),
        tool_use_line("Read", "/src/lib.rs"),
        tool_result_line("pub fn run() {}"),
    ];

    let dir = TempDir::new().unwrap();
    let path = write_jsonl(&dir, "s.jsonl", &lines.iter().map(|s| s.as_str()).collect::<Vec<_>>());

    // Act — via parse_journal (file replay)
    let file_state = parse_journal(&path, 0, None);

    // Act — via process_line (live streaming)
    let mut live_state = JournalState::default();
    for line in &lines {
        process_line(&mut live_state, line);
    }

    // Assert — ambos os caminhos devem produzir o mesmo número de entradas
    assert_eq!(
        file_state.entries.len(),
        live_state.entries.len(),
        "parse_journal e process_line produziram resultados diferentes"
    );
}

#[test]
fn should_resume_from_prev_file_size_without_reprocessing_old_lines() {
    // Seed — escrever primeira linha
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("s.jsonl");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "{}", assistant_line("First message")).unwrap();
    }
    let first_state = parse_journal(&path, 0, None);
    let first_size = first_state.file_size;

    // Seed — adicionar segunda linha
    {
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, "{}", assistant_line("Second message")).unwrap();
    }

    // Act — resumir do offset anterior
    let resumed_state = parse_journal(&path, first_size, Some(&first_state));

    // Assert — deve ter as duas entradas, não duplicar a primeira
    assert_eq!(resumed_state.entries.len(), 2);
    assert_eq!(resumed_state.entries[0].text.as_deref(), Some("First message"));
    assert_eq!(resumed_state.entries[1].text.as_deref(), Some("Second message"));
}
```

### `commands/tasks.rs` ❌ (inexistente — módulo a ser criado)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn todo_write_line(todos: &[(&str, &str, &str)]) -> String {
        let items: Vec<String> = todos.iter().map(|(id, content, status)| {
            format!(r#"{{"id":"{id}","content":"{content}","status":"{status}"}}"#)
        }).collect();
        format!(
            r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","name":"TodoWrite","id":"toolu_01","input":{{"todos":[{}]}}}}],"usage":{{"input_tokens":5,"output_tokens":2,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}}}}"#,
            items.join(",")
        )
    }

    #[test]
    fn should_return_last_todo_state_from_outputs() {
        // Seed
        let db = Arc::new(DatabaseService::open_in_memory().unwrap());
        let id = db.create_session(None, None, "/tmp", "ignore", None).unwrap();

        // Seed — primeiro TodoWrite (estado inicial)
        db.insert_output(id, &todo_write_line(&[
            ("1", "Implement feature", "pending"),
            ("2", "Write tests", "pending"),
        ])).unwrap();

        // Seed — segundo TodoWrite (estado atualizado)
        db.insert_output(id, &todo_write_line(&[
            ("1", "Implement feature", "completed"),
            ("2", "Write tests", "in_progress"),
        ])).unwrap();

        // Act
        let tasks = extract_tasks_from_db(&db, id); // função a ser extraída

        // Assert — deve refletir o ÚLTIMO TodoWrite
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].status, "completed");
        assert_eq!(tasks[1].status, "in_progress");
    }

    #[test]
    fn should_exclude_deleted_todos() {
        // Seed
        let db = Arc::new(DatabaseService::open_in_memory().unwrap());
        let id = db.create_session(None, None, "/tmp", "ignore", None).unwrap();
        db.insert_output(id, &todo_write_line(&[
            ("1", "Active task", "pending"),
            ("2", "Deleted task", "deleted"),
        ])).unwrap();

        // Act
        let tasks = extract_tasks_from_db(&db, id);

        // Assert
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].subject, "Active task");
    }

    #[test]
    fn should_return_empty_when_no_todo_write_in_outputs() {
        // Seed
        let db = Arc::new(DatabaseService::open_in_memory().unwrap());
        let id = db.create_session(None, None, "/tmp", "ignore", None).unwrap();
        db.insert_output(id, r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hi"}],"usage":{"input_tokens":5,"output_tokens":2,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#).unwrap();

        // Act
        let tasks = extract_tasks_from_db(&db, id);

        // Assert
        assert!(tasks.is_empty());
    }
}
```

### `commands/plugins.rs` ❌ (inexistente — módulo a ser criado)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn write_skill_file(dir: &TempDir, name: &str, description: &str) -> std::path::PathBuf {
        let skill_dir = dir.path().join("skills").join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let skill_file = skill_dir.join("SKILL.md");
        std::fs::write(&skill_file, format!(
            "---\nname: {name}\ndescription: {description}\n---\n\n# Content"
        )).unwrap();
        skill_file
    }

    #[test]
    fn should_parse_skill_name_from_frontmatter() {
        let content = "---\nname: my-skill\ndescription: Does something\n---\n";
        let result = frontmatter_field(content, "name");
        assert_eq!(result, Some("my-skill".to_string()));
    }

    #[test]
    fn should_return_none_when_field_absent_from_frontmatter() {
        let content = "---\nname: my-skill\n---\n";
        let result = frontmatter_field(content, "description");
        assert!(result.is_none());
    }

    #[test]
    fn should_truncate_description_at_80_chars_without_breaking_utf8() {
        // Seed — descrição com caractere multi-byte perto do limite
        let desc = "A".repeat(76) + "é" + "BB"; // 'é' ocupa 2 bytes

        // Act
        let truncated = truncate_desc(desc);

        // Assert — não deve panicar e deve ser válido UTF-8
        assert!(truncated.ends_with("..."));
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
    }

    #[test]
    fn should_scan_plugin_dir_and_return_skills() {
        // Seed
        let dir = TempDir::new().unwrap();
        write_skill_file(&dir, "commit", "Create git commits");

        let mut out = Vec::new();

        // Act
        scan_plugin(dir.path(), "superpowers", &mut out);

        // Assert
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].cmd, "/superpowers:commit");
        assert_eq!(out[0].category, "skill");
    }
}
```

### `services/session_manager.rs` — testes adicionais

```rust
#[test]
fn should_not_duplicate_journal_entries_on_restore_from_db() {
    // Seed
    let db = Arc::new(DatabaseService::open_in_memory().unwrap());
    let id = db.create_session(None, None, "/tmp", "ignore", None).unwrap();
    db.insert_output(id, &assistant_line("Hello")).unwrap();

    let mut sm = SessionManager::new(Arc::clone(&db));

    // Act — restaurar duas vezes (não deve duplicar)
    sm.restore_from_db();
    sm.restore_from_db();

    // Assert
    let journal = sm.get_journal(id);
    assert_eq!(journal.len(), 1, "restore_from_db duplicou as entradas");
}

#[test]
fn should_track_token_usage_after_restore() {
    // Seed
    let db = Arc::new(DatabaseService::open_in_memory().unwrap());
    let id = db.create_session(None, None, "/tmp", "ignore", None).unwrap();
    db.insert_output(id, r#"{"type":"assistant","timestamp":"2026-01-01T00:00:00Z","message":{"model":"claude-sonnet-4-6","content":[{"type":"text","text":"Hi"}],"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":2,"cache_read_input_tokens":1}}}"#).unwrap();

    // Act
    let mut sm = SessionManager::new(Arc::clone(&db));
    sm.restore_from_db();

    // Assert
    let sessions = sm.get_sessions();
    let tokens = sessions[0].tokens.as_ref().expect("tokens ausentes após restore");
    assert_eq!(tokens.output, 5);
}
```

---

## Anti-Patterns a Evitar

### ❌ Teste que verifica múltiplos comportamentos

```rust
// RUIM — falha na primeira assertion oculta as demais
#[test]
fn test_session() {
    let db = make_db();
    let id = seed_session(&db);
    db.update_session_status(id, "running").unwrap();
    let sessions = db.get_sessions().unwrap();
    assert_eq!(sessions[0].status, "running");  // se falhar, os demais não rodam
    assert_eq!(sessions[0].id, id);
    assert!(sessions[0].pid.is_none());
}

// BOM — três testes independentes
#[test]
fn should_update_status_to_running() { /* ... */ }

#[test]
fn should_preserve_session_id_after_status_update() { /* ... */ }

#[test]
fn should_have_no_pid_initially() { /* ... */ }
```

### ❌ Unwrap sem contexto no setup

```rust
// RUIM
let db = DatabaseService::open_in_memory().unwrap();

// BOM
let db = DatabaseService::open_in_memory()
    .expect("test setup: failed to open in-memory database");
```

### ❌ Nomes genéricos

```rust
// RUIM
#[test]
fn test_create() { /* ... */ }

// BOM
#[test]
fn should_create_project_and_return_generated_id() { /* ... */ }
```

### ❌ Estado compartilhado entre testes

```rust
// RUIM — static compartilhado é corrida de dados entre testes paralelos
static DB: OnceLock<DatabaseService> = OnceLock::new();

// BOM — cada teste cria o próprio
fn make_db() -> DatabaseService {
    DatabaseService::open_in_memory().expect("...")
}
```

---

## Executando os Testes

```bash
# Todos os testes Rust
npm run test:rust

# Um módulo específico
cargo test -p orbit services::database

# Com output de println! (útil para debug de seed)
cargo test -p orbit -- --nocapture

# Testes em paralelo (padrão) vs sequencial
cargo test -p orbit -- --test-threads=1
```

---

## Checklist Antes de Fazer Commit com Testes Novos

- [ ] Cada teste tem comentários `// Seed`, `// Act`, `// Assert` (e `// Cleanup` se explícito)
- [ ] Nome do teste descreve o comportamento esperado
- [ ] Nenhum teste depende de arquivo externo fixo (usar `TempDir`)
- [ ] Nenhum `unwrap()` sem `expect("contexto")` no setup
- [ ] Testes de erro verificam **o tipo ou a mensagem** do erro, não apenas que ele ocorreu
- [ ] `parse_journal` e `process_line` têm testes para o mesmo input (paridade garantida)
