# Session Nickname & Git Worktree Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Adicionar campo de apelido (com auto-geração de nome Android) e opção de git worktree no modal de criação de sessão, fazendo o Claude rodar dentro do worktree quando ativado.

**Architecture:** No frontend, um novo utilitário de nomes Android preenche automaticamente o campo de apelido a partir do caminho do projeto. No backend Rust, um novo módulo `worktree` cria o `git worktree` quando solicitado e o `session_manager` passa o caminho do worktree como `cwd` ao spawnar o Claude.

**Tech Stack:** Svelte 5 + TypeScript (frontend), Rust 1.85 + Tauri 2 (backend), SQLite (já tem colunas `worktree_path` e `branch_name`), `git` CLI via `std::process::Command`.

---

## Mapa de arquivos

| Ação | Arquivo |
|------|---------|
| Criar | `api/lib/android-names.ts` |
| Criar | `api/lib/android-names.test.ts` |
| Criar | `front/src/services/worktree.rs` |
| Modificar | `api/lib/stores/sessions.ts` |
| Modificar | `api/lib/tauri.ts` |
| Modificar | `api/lib/mock/tauri-mock.ts` |
| Modificar | `api/components/NewSessionModal.svelte` |
| Modificar | `front/src/services/mod.rs` |
| Modificar | `front/src/services/database.rs` |
| Modificar | `front/src/services/session_manager.rs` |
| Modificar | `front/src/ipc/session.rs` |

---

### Task 1: Utilitário de nomes Android (Frontend)

**Files:**
- Create: `api/lib/android-names.ts`
- Create: `api/lib/android-names.test.ts`

- [ ] **Step 1: Criar o arquivo de nomes**

Criar `api/lib/android-names.ts` com o seguinte conteúdo:

```typescript
const ANDROID_CODENAMES = [
  'hammerhead', 'shamu', 'bullhead', 'angler', 'marlin', 'sailfish',
  'walleye', 'taimen', 'blueline', 'crosshatch', 'flame', 'coral',
  'sunfish', 'redfin', 'barbet', 'oriole', 'raven', 'cheetah',
  'panther', 'lynx', 'felix', 'akita', 'caiman', 'komodo', 'tokay',
  'dolph', 'husky', 'shiba', 'tangor', 'comet',
];

export function generateSessionName(projectName: string): string {
  const codename = ANDROID_CODENAMES[Math.floor(Math.random() * ANDROID_CODENAMES.length)];
  return `${codename} · ${projectName}`;
}
```

- [ ] **Step 2: Escrever os testes**

Criar `api/lib/android-names.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { generateSessionName } from './android-names';

describe('generateSessionName', () => {
  it('inclui o nome do projeto na saída', () => {
    const name = generateSessionName('my-project');
    expect(name).toContain('my-project');
  });

  it('segue o padrão "<codename> · <project>"', () => {
    const name = generateSessionName('orbit');
    expect(name).toMatch(/^[a-z]+ · orbit$/);
  });

  it('gera resultados variados (probabilístico com 20 amostras)', () => {
    const names = new Set(Array.from({ length: 20 }, () => generateSessionName('p')));
    expect(names.size).toBeGreaterThan(1);
  });
});
```

- [ ] **Step 3: Rodar os testes**

```bash
npm run test -- android-names
```

Expected: 3 testes passando.

- [ ] **Step 4: Commit**

```bash
git add api/lib/android-names.ts api/lib/android-names.test.ts
git commit -m "feat: add Android codename generator for session names"
```

---

### Task 2: Atualizar tipos TypeScript e mock

**Files:**
- Modify: `api/lib/stores/sessions.ts`
- Modify: `api/lib/mock/tauri-mock.ts`
- Modify: `api/lib/tauri.ts`

- [ ] **Step 1: Adicionar `worktreePath` e `branchName` à interface `Session`**

Em `api/lib/stores/sessions.ts`, adicionar dois campos à interface `Session` (após `gitBranch`):

```typescript
  gitBranch: string | null;
  worktreePath: string | null;   // <-- adicionar
  branchName: string | null;     // <-- adicionar
  tokens: TokenUsage | null;
```

- [ ] **Step 2: Adicionar `useWorktree` ao `CreateSessionOptions` em `api/lib/tauri.ts`**

Na interface `CreateSessionOptions` em `api/lib/tauri.ts`:

```typescript
export interface CreateSessionOptions {
  projectPath: string;
  prompt: string;
  model?: string;
  permissionMode?: 'ignore' | 'approve';
  sessionName?: string;
  useWorktree?: boolean;  // <-- adicionar
}
```

E na função `createSession`, adicionar o novo campo no `invoke`:

```typescript
export async function createSession(opts: CreateSessionOptions): Promise<Session> {
  return await invoke('create_session', {
    projectPath: opts.projectPath,
    prompt: opts.prompt,
    model: opts.model ?? null,
    permissionMode: opts.permissionMode ?? 'ignore',
    sessionName: opts.sessionName ?? null,
    useWorktree: opts.useWorktree ?? false,  // <-- adicionar
  });
}
```

- [ ] **Step 3: Atualizar os mocks em `api/lib/mock/tauri-mock.ts`**

Em cada objeto da array `MOCK_SESSIONS`, adicionar os dois campos novos (o TypeScript vai reclamar em svelte-check se não forem adicionados):

```typescript
// Em cada sessão mock (há 3):
worktreePath: null,
branchName: null,
```

Procurar pelas 3 ocorrências de `costUsd: null,` e adicionar os campos logo após cada uma:

```typescript
    costUsd: null,
    worktreePath: null,
    branchName: null,
```

- [ ] **Step 4: Verificar que svelte-check passa**

```bash
npx svelte-check --tsconfig tsconfig.json 2>&1 | tail -5
```

Expected: `0 errors, 0 warnings`.

- [ ] **Step 5: Commit**

```bash
git add api/lib/stores/sessions.ts api/lib/tauri.ts api/lib/mock/tauri-mock.ts
git commit -m "feat: add worktreePath/branchName to Session type and useWorktree to CreateSessionOptions"
```

---

### Task 3: NewSessionModal — campo de apelido e toggle de worktree

**Files:**
- Modify: `api/components/NewSessionModal.svelte`

- [ ] **Step 1: Adicionar import e variáveis**

No bloco `<script>` do componente, após a linha `import { createSession, diagnoseSpawn } from '../lib/tauri';`, adicionar o import:

```typescript
  import { generateSessionName } from '../lib/android-names';
```

E adicionar as variáveis de estado logo após `let diag: SpawnDiagnostic | null = null;`:

```typescript
  let nickname = '';
  let generatedName = '';
  let useWorktree = false;

  // Regenera o nome sugerido toda vez que o path muda
  $: if (path) {
    const projName = path.split(/[/\\]/).filter(Boolean).pop() ?? 'sessão';
    generatedName = generateSessionName(projName);
  }
```

- [ ] **Step 2: Atualizar a função `submit`**

Substituir a chamada `createSession` atual pela versão que passa o apelido e o worktree. Substituir o bloco `try` inteiro dentro de `submit()`:

```typescript
  async function submit() {
    if (!path.trim()) {
      error = 'project path required';
      return;
    }
    const projName = path.split(/[/\\]/).filter(Boolean).pop() ?? 'sessão';
    const finalName = nickname.trim() || generatedName || generateSessionName(projName);
    loading = true;
    error = '';
    try {
      await createSession({
        projectPath: path.trim(),
        prompt: prompt.trim() || 'Hello',
        model: model === 'auto' ? undefined : model,
        permissionMode: 'ignore',
        sessionName: finalName,
        useWorktree,
      });
      dispatch('done');
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      loading = false;
    }
  }
```

- [ ] **Step 3: Adicionar os campos ao template**

Após o bloco `<div class="row">` que contém o seletor de modelo (termina em `</div>` antes do bloco `{#if error}`), adicionar dois novos campos:

```svelte
    <div class="field">
      <label class="label" for="ns-nickname">apelido</label>
      <input
        id="ns-nickname"
        class="input"
        bind:value={nickname}
        placeholder={generatedName || 'selecione um projeto para sugerir nome'}
        disabled={loading}
      />
    </div>

    <label class="toggle-row">
      <input type="checkbox" bind:checked={useWorktree} disabled={loading} />
      <span class="toggle-label">criar git worktree</span>
    </label>
```

- [ ] **Step 4: Adicionar estilos para o toggle**

No bloco `<style>`, adicionar ao final (antes do `</style>`):

```css
  .toggle-row {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    user-select: none;
  }
  .toggle-row input[type='checkbox'] {
    accent-color: var(--ac);
    width: 14px;
    height: 14px;
    cursor: pointer;
  }
  .toggle-label {
    font-size: var(--sm);
    color: var(--t1);
  }
```

- [ ] **Step 5: Verificar lint e svelte-check**

```bash
npm run lint 2>&1 | tail -10
```

Expected: sem erros.

- [ ] **Step 6: Commit**

```bash
git add api/components/NewSessionModal.svelte
git commit -m "feat: add nickname field and worktree toggle to NewSessionModal"
```

---

### Task 4: Backend — módulo worktree Rust

**Files:**
- Create: `front/src/services/worktree.rs`
- Modify: `front/src/services/mod.rs`

- [ ] **Step 1: Escrever os testes primeiro**

Criar `front/src/services/worktree.rs` com apenas os testes (o código real virá no passo 3):

```rust
use std::path::PathBuf;

pub fn generate_branch_slug(name: &str) -> String {
    todo!()
}

pub fn create_worktree(_project_path: &std::path::Path, _slug: &str) -> Result<PathBuf, String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_lowercase() {
        assert_eq!(generate_branch_slug("My Session"), "my-session");
    }

    #[test]
    fn test_slug_special_chars() {
        // O · do nome Android vira hífen
        assert_eq!(generate_branch_slug("hammerhead · orbit"), "hammerhead-orbit");
    }

    #[test]
    fn test_slug_collapses_dashes() {
        assert_eq!(generate_branch_slug("  spaces  "), "spaces");
    }

    #[test]
    fn test_slug_preserves_hyphens() {
        assert_eq!(generate_branch_slug("abc-def"), "abc-def");
    }
}
```

- [ ] **Step 2: Adicionar `pub mod worktree;` em `front/src/services/mod.rs`**

O arquivo atual é:
```rust
pub mod database;
pub mod session_manager;
pub mod spawn_manager;
```

Adicionar:
```rust
pub mod database;
pub mod session_manager;
pub mod spawn_manager;
pub mod worktree;
```

- [ ] **Step 3: Rodar os testes para confirmar que falham**

```bash
cd front && cargo test worktree 2>&1 | tail -10
```

Expected: 4 testes com `panicked at 'not yet implemented'`.

- [ ] **Step 4: Implementar `generate_branch_slug` e `create_worktree`**

Substituir todo o conteúdo de `front/src/services/worktree.rs` pela implementação completa:

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

/// Converte o nome da sessão em um slug válido para branch git.
/// "hammerhead · orbit" → "hammerhead-orbit"
pub fn generate_branch_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Cria um git worktree em `<project_path>/.worktrees/<slug>` no branch `orbit/<slug>`.
/// Retorna o caminho absoluto do worktree criado.
pub fn create_worktree(project_path: &Path, slug: &str) -> Result<PathBuf, String> {
    let worktree_path = project_path.join(".worktrees").join(slug);
    let branch_name = format!("orbit/{slug}");

    #[cfg(windows)]
    let output = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        Command::new("git")
            .args([
                "-C",
                project_path.to_str().unwrap_or("."),
                "worktree",
                "add",
                worktree_path.to_str().unwrap_or(""),
                "-b",
                &branch_name,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("git não encontrado: {e}"))?
    };

    #[cfg(not(windows))]
    let output = Command::new("git")
        .args([
            "-C",
            project_path.to_str().unwrap_or("."),
            "worktree",
            "add",
            worktree_path.to_str().unwrap_or(""),
            "-b",
            &branch_name,
        ])
        .output()
        .map_err(|e| format!("git não encontrado: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(worktree_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_lowercase() {
        assert_eq!(generate_branch_slug("My Session"), "my-session");
    }

    #[test]
    fn test_slug_special_chars() {
        assert_eq!(generate_branch_slug("hammerhead · orbit"), "hammerhead-orbit");
    }

    #[test]
    fn test_slug_collapses_dashes() {
        assert_eq!(generate_branch_slug("  spaces  "), "spaces");
    }

    #[test]
    fn test_slug_preserves_hyphens() {
        assert_eq!(generate_branch_slug("abc-def"), "abc-def");
    }

    /// Teste de integração: cria um worktree real em um repo git temporário.
    /// Requer o binário `git` instalado (disponível em CI/Windows).
    #[test]
    fn test_create_worktree_in_real_git_repo() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let repo = dir.path();

        // Inicializa um repo git limpo
        let init = Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        assert!(init.status.success(), "git init falhou");

        // Commit vazio (necessário para worktree)
        for cmd in [
            vec!["config", "user.email", "test@test.com"],
            vec!["config", "user.name", "Test"],
            vec!["commit", "--allow-empty", "-m", "init"],
        ] {
            let out = Command::new("git").args(&cmd).current_dir(repo).output().unwrap();
            assert!(out.status.success(), "git {:?} falhou", cmd);
        }

        let result = create_worktree(repo, "minha-sessao");
        assert!(result.is_ok(), "create_worktree falhou: {:?}", result.err());

        let wt_path = result.unwrap();
        assert!(wt_path.exists(), "worktree path não existe: {:?}", wt_path);
        assert!(wt_path.join(".git").exists(), "worktree sem .git");
    }
}
```

**Nota:** o teste de integração usa `tempfile`. Verificar se `tempfile` já está em `front/Cargo.toml` na seção `[dev-dependencies]`. Se não estiver, adicionar:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 5: Rodar os testes**

```bash
cd front && cargo test worktree 2>&1 | tail -15
```

Expected: 5 testes passando (4 de slug + 1 de integração).

- [ ] **Step 6: Commit**

```bash
git add front/src/services/worktree.rs front/src/services/mod.rs front/Cargo.toml
git commit -m "feat(rust): add worktree service with git worktree creation"
```

---

### Task 5: Backend — database e session_manager

**Files:**
- Modify: `front/src/services/database.rs`
- Modify: `front/src/services/session_manager.rs`

- [ ] **Step 1: Adicionar `update_session_worktree` em `database.rs`**

Após o método `update_session_pid` (em torno da linha 138), adicionar:

```rust
    pub fn update_session_worktree(
        &self,
        id: SessionId,
        worktree_path: &str,
        branch_name: &str,
    ) -> SqlResult<()> {
        self.conn.lock().unwrap().execute(
            "UPDATE sessions SET worktree_path = ?1, branch_name = ?2, \
             updated_at = datetime('now') WHERE id = ?3",
            params![worktree_path, branch_name, id],
        )?;
        Ok(())
    }
```

- [ ] **Step 2: Atualizar `init_session` em `session_manager.rs`**

A assinatura atual é:

```rust
pub fn init_session(
    &mut self,
    project_path: &str,
    session_name: Option<&str>,
    permission_mode: &str,
    model: Option<&str>,
) -> Result<Session, String>
```

Substituir pela nova assinatura com `use_worktree`:

```rust
pub fn init_session(
    &mut self,
    project_path: &str,
    session_name: Option<&str>,
    permission_mode: &str,
    model: Option<&str>,
    use_worktree: bool,
) -> Result<Session, String>
```

Após a linha `let session_id = self.db.create_session(...)?;` (cria o registro no DB), mas antes de construir o `Session` struct, adicionar o bloco de criação do worktree:

```rust
        let (worktree_path_val, branch_name_val) = if use_worktree {
            let slug = crate::services::worktree::generate_branch_slug(
                session_name.unwrap_or(&project_name),
            );
            let wt_path = crate::services::worktree::create_worktree(
                std::path::Path::new(project_path),
                &slug,
            )?;
            let branch = format!("orbit/{slug}");
            let wt_str = wt_path.to_string_lossy().to_string();
            let _ = self.db.update_session_worktree(session_id, &wt_str, &branch);
            (Some(wt_str), Some(branch))
        } else {
            (None, None)
        };
```

E atualizar a construção do `Session` struct, substituindo:

```rust
            worktree_path: None,
            branch_name: None,
```

por:

```rust
            worktree_path: worktree_path_val,
            branch_name: branch_name_val,
```

- [ ] **Step 3: Atualizar `do_spawn` para usar `worktree_path` como cwd**

Em `do_spawn`, na seção que extrai os campos da sessão ativa (torno da linha 126), substituir:

```rust
                a.session.cwd.clone().unwrap_or_default(),
```

por:

```rust
                a.session
                    .worktree_path
                    .clone()
                    .or_else(|| a.session.cwd.clone())
                    .unwrap_or_default(),
```

- [ ] **Step 4: Atualizar os testes existentes em `session_manager.rs`**

Todos os calls a `init_session` nos testes passam 4 argumentos. Adicionar `false` como 5º argumento em todos eles. Buscar `init_session(` no arquivo e adicionar `, false` antes de cada `)` de fechamento:

```rust
// Antes (exemplo):
.init_session("/tmp/proj", None, "ignore", None)
// Depois:
.init_session("/tmp/proj", None, "ignore", None, false)
```

Há 6 ocorrências (nas funções `test_init_session_creates_db_record`, `test_init_session_populates_journal_state`, `test_init_populates_active`, `test_stop_session_updates_db`, `test_delete_removes_from_active_and_state`, e a passagem direta do DB em `test_restore_from_db_rebuilds_journal`).

- [ ] **Step 5: Rodar os testes**

```bash
cd front && cargo test 2>&1 | tail -15
```

Expected: todos os testes passando, incluindo os da `session_manager`.

- [ ] **Step 6: Commit**

```bash
git add front/src/services/database.rs front/src/services/session_manager.rs
git commit -m "feat(rust): integrate worktree creation into session init; use worktree path as cwd"
```

---

### Task 6: Backend — comando IPC `create_session`

**Files:**
- Modify: `front/src/ipc/session.rs`

- [ ] **Step 1: Adicionar `use_worktree` ao comando `create_session`**

Em `front/src/ipc/session.rs`, a função `create_session` atualmente tem os parâmetros:

```rust
pub fn create_session(
    project_path: String,
    prompt: String,
    model: Option<String>,
    permission_mode: Option<String>,
    session_name: Option<String>,
    state: State<SessionState>,
    app: AppHandle,
) -> Result<Session, String>
```

Adicionar `use_worktree: Option<bool>` após `session_name`:

```rust
#[tauri::command]
pub fn create_session(
    project_path: String,
    prompt: String,
    model: Option<String>,
    permission_mode: Option<String>,
    session_name: Option<String>,
    use_worktree: Option<bool>,
    state: State<SessionState>,
    app: AppHandle,
) -> Result<Session, String> {
    let mode = permission_mode.unwrap_or_else(|| "ignore".to_string());

    let session = {
        let mut m = state.0.lock().unwrap();
        m.init_session(
            &project_path,
            session_name.as_deref(),
            &mode,
            model.as_deref(),
            use_worktree.unwrap_or(false),
        )?
    };

    use tauri::Emitter;
    let _ = app.emit("session:created", &session);

    let manager = Arc::clone(&state.0);
    let session_id = session.id;
    std::thread::spawn(move || {
        SessionManager::do_spawn(manager, app, session_id, prompt);
    });

    Ok(session)
}
```

- [ ] **Step 2: Rodar clippy e testes**

```bash
cd front && cargo clippy -- -D warnings 2>&1 | tail -10 && cargo test 2>&1 | tail -10
```

Expected: 0 warnings, todos os testes passando.

- [ ] **Step 3: Rodar lint completo do frontend**

```bash
npm run lint 2>&1 | tail -10
```

Expected: sem erros.

- [ ] **Step 4: Commit final**

```bash
git add front/src/ipc/session.rs
git commit -m "feat(rust): expose use_worktree parameter in create_session IPC command"
```

---

## Self-review checklist

- [x] **Spec coverage:**
  - Apelido no modal: Task 1 (gerador) + Task 3 (UI)
  - Nome automático se campo vazio: Task 3 Step 2 (`finalName = nickname.trim() || generatedName || ...`)
  - Toggle de worktree: Task 3 Step 3
  - Criação do worktree: Task 4 Step 4
  - Claude forçado a rodar no worktree: Task 5 Step 3 (`do_spawn` usa `worktree_path` como cwd)
  - Persistência no DB: Task 5 Step 1 (`update_session_worktree`) — schema já tem colunas

- [x] **Sem placeholders:** todos os steps têm código completo.

- [x] **Consistência de tipos:**
  - `useWorktree: bool` (Rust) → `useWorktree: boolean` (TS) — nomes consistentes
  - `worktreePath`/`branchName` adicionados na Task 2 e usados na Task 5
  - `generate_branch_slug` criado na Task 4 e usado na Task 5 como `crate::services::worktree::generate_branch_slug`
