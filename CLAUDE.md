# Orbit — CLAUDE.md

Guia de referência para o Claude Code trabalhar neste repositório.

---

## O que é o Orbit

Orbit é um **dashboard desktop para gerenciar múltiplas sessões do Claude Code em paralelo**, construído com Tauri 2 (Rust + Svelte). Permite criar sessões, acompanhar output em tempo real, visualizar diffs de arquivos, tasks e tokens consumidos.

- Plataformas: **Windows 10 1903+**, **Ubuntu 22.04+** (e outras distros Linux com webkit2gtk 4.1), **macOS** (Intel e Apple Silicon)
- Identificador: `com.josefernando.orbit`
- Repositório: `github.com/xinnaider/orbit`

---

## Stack

| Camada | Tecnologia |
|--------|-----------|
| Desktop framework | Tauri 2.x |
| Backend | Rust 1.85 (MSRV) |
| Frontend | SvelteKit 2.9 + Svelte 5 + TypeScript 5.6 |
| Bundler | Vite 6 (porta 1420) |
| Banco de dados | SQLite via rusqlite 0.31 |
| Testes Rust | cargo test (integração, sem mocks de DB) |
| Testes TS | Vitest 2 |
| Lint TS/Svelte | ESLint 9 + eslint-plugin-svelte |
| Lint Rust | cargo clippy (-D warnings) |
| Format TS/Svelte | Prettier 3.8 + prettier-plugin-svelte |
| Format Rust | rustfmt (tauri/rustfmt.toml) |

---

## Estrutura de diretórios

```
agent-dashboard-v2/
├── tauri/                      # Backend Rust / Tauri
│   ├── src/
│   │   ├── main.rs             # Entry point mínimo
│   │   ├── lib.rs              # Inicialização do app Tauri, plugins, IPC handlers
│   │   ├── models.rs           # Tipos centrais: Session, JournalEntry, AgentStatus, etc.
│   │   ├── commands.rs         # Comandos Tauri auxiliares (diff, tasks, slash commands, files)
│   │   ├── journal_reader.rs   # Parser JSONL do output do Claude Code (incremental)
│   │   ├── agent_tree.rs       # Leitura de metadados de subagentes (.meta.json)
│   │   ├── diff_builder.rs     # Diff de versões de arquivo (Myers LCS)
│   │   ├── ipc/
│   │   │   ├── session.rs      # Comandos Tauri de sessão (create, stop, list, send_message)
│   │   │   └── project.rs      # Comandos Tauri de projeto (create, list)
│   │   └── services/
│   │       ├── database.rs     # Wrapper SQLite com migrations automáticas
│   │       ├── session_manager.rs  # Estado em memória + spawn + eventos Tauri
│   │       └── spawn_manager.rs    # Spawn do claude CLI, captura stdout/stderr
│   ├── Cargo.toml
│   ├── tauri.conf.json         # Configuração Tauri (janela 1200×750, segurança)
│   ├── rustfmt.toml            # max_width=100, tab_spaces=4
│   └── .clippy.toml            # cognitive-complexity=30, too-many-lines=100
│
├── ui/                         # Frontend SvelteKit
│   ├── App.svelte              # Raiz: listeners de eventos Tauri, banners globais
│   ├── app.css                 # Estilos globais (variáveis CSS, temas)
│   ├── routes/                 # Rotas SvelteKit (+page.svelte, +layout.svelte)
│   ├── components/             # 23 componentes Svelte
│   └── lib/
│       ├── tauri.ts            # Wrapper IPC com fallback mock
│       ├── types.ts            # Tipos TypeScript (espelho dos models.rs)
│       ├── status.ts           # Helpers de status/cor
│       ├── cost.ts             # Cálculo de custo por tokens
│       └── stores/             # Svelte stores (sessions, journal, preferences, agents)
│
├── .github/workflows/          # CI: lint + build (Windows)
├── CLAUDE.md                   # Este arquivo
├── CHANGELOG.md                # Histórico de alterações (auto-atualizado no pre-commit)
├── package.json
├── vite.config.js
├── svelte.config.js
├── tsconfig.json
├── eslint.config.js
└── .prettierrc
```

---

## Tipos centrais (models.rs)

```
AgentStatus        Working | Input | Idle | New
SessionStatus      Initializing | Running | Waiting | Completed | Stopped | Error
JournalEntryType   User | Thinking | Assistant | ToolCall | ToolResult | System | Progress
Session            id, status, model, pid, cwd, tokens, contextPercent, pendingApproval, miniLog
JournalEntry       sessionId, timestamp, entryType, text, thinking, tool, toolInput, output
TokenUsage         input, output, cacheRead, cacheWrite
MiniLogEntry       tool, target, result, success
TaskItem           id, subject, description, status (pending|in_progress|completed)
```

---

## Banco de dados (SQLite)

Arquivo: `{AppData}/Local/com.josefernando.orbit/agent-dashboard.db`

```sql
projects        (id, name, path UNIQUE, created_at)
sessions        (id, project_id, name, status, permission_mode, model, pid, cwd,
                 claude_session_id, created_at, updated_at)
session_outputs (id, session_id, data TEXT, created_at)
                 -- Armazena JSONL bruto de cada sessão
```

Migrations são aplicadas automaticamente ao iniciar o app (`database.rs`).

---

## Fluxo de sessão (end-to-end)

```
Usuário cria sessão
  ↓
ipc/session.rs::create_session()
  → Fase 1 (síncrona): cria registro no DB, retorna Session imediatamente
  → Emite `session:created`
  → Spawna thread background com do_spawn()

do_spawn() — background thread
  → spawn_manager::spawn_claude()
     · Busca o binário claude no PATH (npm, pnpm, nvm, .local/bin)
     · Spawna com --output-format stream-json --verbose --dangerously-skip-permissions
     · Captura stdout (JSON lines) + stderr (detecção de rate limit)
  → Emite `session:running` com PID
  → reader_loop(): lê cada linha JSONL
     · Detecta rate limit → emite `session:rate-limit`
     · process_line() → atualiza JournalState
     · Emite `session:output` (nova entrada no feed)
     · Emite `session:state` (tokens, status, contextPercent, pendingApproval)

Mensagem de follow-up
  → send_message() → do_spawn() com --resume <claude_session_id>
```

---

## Eventos Tauri (Rust → Frontend)

| Evento | Payload | Quando |
|--------|---------|--------|
| `session:created` | Session | Sessão criada no DB |
| `session:running` | `{sessionId, pid}` | Claude spawnou com sucesso |
| `session:output` | `{sessionId, entry}` | Nova entrada no journal |
| `session:state` | `{sessionId, status, tokens, contextPercent, pendingApproval, miniLog}` | Estado atualizado |
| `session:stopped` | `{sessionId}` | Sessão parou |
| `session:error` | `{sessionId, error}` | Falha ao spawnar |
| `session:rate-limit` | `{sessionId}` | Rate limit detectado |

---

## Componentes principais

| Componente | Responsabilidade |
|-----------|-----------------|
| `App.svelte` | Listeners de eventos, banners globais (erro, rate limit) |
| `Sidebar.svelte` | Lista de sessões, modal de criação, context menu |
| `CentralPanel.svelte` | Feed da sessão selecionada, header com status |
| `Feed.svelte` | Renderização incremental do journal, auto-scroll |
| `ToolCallEntry.svelte` | Tool calls com diffs, bash output, streaming entries |
| `MetaPanel.svelte` | Painel direito: tokens, custo, context %, tools recentes |
| `InputBar.svelte` | Input de mensagem com slash commands e @ file picker |
| `NewSessionModal.svelte` | Formulário de nova sessão (path, prompt, modelo) |

---

## Modo mock (dev sem backend Rust)

```bash
npm run dev:mock
```

`ui/lib/mock/tauri-mock.ts` simula todos os comandos e eventos Tauri. Use `VITE_MOCK=true` para ativar.

---

## Scripts npm

| Script | O que faz |
|--------|-----------|
| `npm run tauri:dev` | **Dev principal** — Vite + Rust com hot reload |
| `npm run dev:mock` | Frontend sem Rust (mock) |
| `npm run lint` | ESLint + svelte-check + clippy |
| `npm run format` | Prettier + rustfmt (auto-fix) |
| `npm run test` | Vitest (frontend) |
| `npm run test:rust` | cargo test (backend) |
| `npm run tauri:build` | Build de produção (.exe/.msi) |

---

## Convenções de código

### Rust
- Formatação via `rustfmt` — `max_width = 100`, `tab_spaces = 4`
- Clippy com `-D warnings` — zero warnings tolerados
- `Arc<Mutex<T>>` para estado compartilhado entre threads
- Eventos Tauri emitidos com `app.emit(...)` — nomenclatura `domain:event` (kebab-case)
- Testes de integração usam DB real em memória (`DatabaseService::open_in_memory()`)
- Sem mocks de banco — erros de divergência mock/produção já aconteceram

### TypeScript / Svelte
- Prettier com `singleQuote: true`, `semi: true`, `printWidth: 100`
- ESLint com `--max-warnings 0`
- Svelte stores para estado global; props para estado local de componente
- `tauri.ts` como única interface com o backend — nunca usar `invoke` diretamente nos componentes
- Tipos TS espelham os structs Rust (camelCase no TS, snake_case no Rust)

### Git
- Commits in English, conventional prefix: `feat:`, `fix:`, `refactor:`, `chore:`, `docs:`
- Never commit with `--no-verify`
- Issues follow the same conventional prefix format: `feat:`, `fix:`, `refactor:`, `chore:`, `docs:`
- **Commit by logical context, not by step.** The pre-commit hook runs Prettier, rustfmt, ESLint, svelte-check, and Clippy on every commit — committing after each small step wastes tokens and time. Group related changes into one commit per meaningful unit of work (e.g. one commit for a new utility + its tests, one for the new component, one for the wiring). A feature with 3 tasks → 2–3 commits, not 10.
- **Always run checks BEFORE committing.** Do not rely solely on the pre-commit hook. When finishing a logical unit of work, before `git commit`, manually run: `npx prettier --check "ui/**/*.{ts,svelte,css}"`, `npx eslint ui --max-warnings 0`, `npx svelte-check --fail-on-warnings`, `cargo clippy --manifest-path tauri/Cargo.toml -- -D warnings`. Fix any failures before committing.

#### Git hook: `pre-commit`
Roda automaticamente antes de todo commit:
1. **Prettier** auto-formata `ui/**/*.{ts,svelte,css}` e re-adiciona ao stage
2. **rustfmt** auto-formata o código Rust e re-adiciona ao stage
3. **ESLint** com `--max-warnings 0` — bloqueia o commit se falhar
4. **svelte-check** com `--fail-on-warnings` — bloqueia o commit se falhar
5. **Clippy** com `-D warnings` — bloqueia o commit se falhar

#### CHANGELOG Policy

**Before each commit with user-facing changes, update `CHANGELOG.md`.**

The CHANGELOG is written for **users and customers** — not developers. Write as if explaining what changed to someone who uses the app, not someone who reads the code.

**Rules:**
- Plain English
- Describe the *effect* of the change, not *how* it was implemented
- No file names, no unnecessary technical terms, no implementation details
- Group entries by **month and year** (e.g. `## April 2026`)
- Each entry has date and category in the title: `### MM/DD · <Category> — <Title>`
- Categories: **New**, **Improvement**, **Adjustment**, **Fix**
- One short paragraph describing what the user perceives
- Only include what a user would notice: new feature, visible fix, UX improvement
- Ignore internal refactors, lint fixes, config changes that don't affect the user

**Good example:**
```
### 04/06 · New — API rate limit warning
When the Claude API rate limit is reached, the app now shows a clear message
on screen instead of silently stopping.
```

**Bad example:**
```
- fix: detect rate_limit_error in session_manager reader_loop stderr thread
- updated ui/App.svelte to listen for session:rate-limit event
```

---

## Architecture Rules — Provider System

The provider system uses **dependency inversion**: `session_manager` depends on the `Provider` trait, not on concrete implementations. All provider-specific behavior must live behind this trait.

### Zero hardcoded provider strings outside providers/

**Never** compare provider IDs with string literals in `session_manager.rs`, `ipc/`, or `commands/`. If you need provider-specific behavior, add a method to the `Provider` trait.

| Need | Wrong | Right |
|------|-------|-------|
| Format model ID | `if pid != "claude-code" && pid != "codex"` | `provider.format_model(raw)` |
| Get line parser | `match id { "claude-code" => ... }` | `provider.line_processor()` |
| Check capability | `if provider == "claude-code"` | `provider.supports_effort()` |
| Default provider | `"claude-code".to_string()` | Constant: `Provider::DEFAULT_ID` |

**Adding a new provider should require:**
1. Create `providers/<name>.rs` implementing `Provider`
2. Register in `lib.rs` with `registry.register(...)`
3. Done — no changes to `session_manager`, `ipc/session`, or `commands/`

**Current violations to fix** (tech debt):
- `session_manager.rs:219` — model prefix logic hardcoded (needs `fn format_model`)
- `session_manager.rs:388,786` — line_processor dispatch (needs `fn line_processor`)
- `commands/providers.rs:39-128` — manual backend list (needs `registry.all()`)
- Frontend: 6 components compare `provider === 'claude-code'` (needs capabilities from backend)

### Provider Trait — Required Methods

```rust
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn spawn(&self, config: ProviderSpawnConfig) -> Result<SpawnHandle, String>;
    fn process_line(&self, state: &mut JournalState, line: &str);
    fn line_processor(&self) -> fn(&mut JournalState, &str);  // fn pointer for Send threads
    fn format_model(&self, raw_model: &str) -> String;        // provider-specific model formatting
    fn context_window(&self, model: &str) -> Option<u64>;
    fn slash_commands(&self) -> Vec<SlashCommand>;
    fn supports_effort(&self) -> bool;
    fn supports_ssh(&self) -> bool;
    fn cli_name(&self) -> &str;
    fn find_cli(&self) -> Option<String>;
    fn install_hint(&self) -> &str;
}
```

### SSH Spawning

SSH wrapping lives in `services/ssh.rs`. Providers must **not** call `posix_escape` on their arguments — `spawn_via_ssh` handles all escaping in a single layer (`bash -lc "script"`).

- Providers build the remote command string with **raw** values
- `spawn_via_ssh` wraps everything once with proper escaping
- Env vars use inline syntax (`KEY=val cmd args`), not `export`
- Double quotes are used for the outer `bash -lc` wrapper; `$`, `` ` ``, `\`, `"` are escaped

### Encrypted Secrets

API keys and SSH passwords are stored AES-256-GCM encrypted in the database (`api_key_enc`, `ssh_password_enc` columns). The encryption key is at `{app_data}/orbit.key`.

- **Never** store plaintext credentials in the DB
- Always use `db.save_session_secrets()` / `db.load_session_secrets()`
- SSH password and API key must be set **before** the spawn thread starts (avoid race conditions)

### Rust Best Practices (Apollo Style)

- **Borrow over clone**: prefer `&str` over `String`, `&[T]` over `Vec<T>` in function params
- **No `unwrap()`/`expect()` in production** — use `?`, `let Ok(..) = .. else`, or `unwrap_or_else`
- **`thiserror` for typed errors** — no `anyhow` in library code, no `format!("error: {e}")` as error type
- **Static dispatch by default** (`impl Trait`, `<T: Trait>`) — use `dyn Trait` only for heterogeneous collections (like `ProviderRegistry`)
- **Iterators over loops** when transforming collections — no intermediate `.collect()` unless needed
- **`?` for error propagation** — no verbose `match` chains for Result/Option
- **Comments explain *why***, not *what* — let naming and structure speak for themselves

---

## Self-Improvement Loop

**No início de cada sessão:** leia `docs/lessons.md` se existir e aplique as lições registradas.

**Após qualquer correção do usuário:**
1. Identifique o padrão do erro (não apenas o caso específico)
2. Adicione uma entrada em `docs/lessons.md` com:
   - **Regra**: o que fazer (ou não fazer)
   - **Por quê**: motivação / o que deu errado
   - **Quando aplicar**: contexto em que a regra vale
3. Itere nas lições existentes se o mesmo erro se repetir — refine a regra, não apenas acumule entradas

O arquivo `docs/lessons.md` é versionado no repositório para que todos os colaboradores e agentes se beneficiem das lições aprendidas.

---

## Specs de features

Toda feature nova deve ter uma spec em `docs/specs/` antes de ser implementada.

- Nome do arquivo: `docs/specs/<nome-da-feature>.md` (kebab-case)
- Conteúdo mínimo: objetivo, comportamento esperado, casos de borda, critérios de aceitação
- A spec deve ser criada ou atualizada **antes** de escrever código
- Ao iniciar uma sessão em uma branch de feature, leia a spec correspondente se existir

---

## CI (GitHub Actions)

**Lint job** (todo PR/push):
1. `cargo fmt --check`
2. `prettier --check`
3. `cargo clippy -- -D warnings`
4. `eslint + svelte-check`

**Build jobs** (após lint passar):
- **Windows** (`windows-latest`): `npm run tauri:build` → `.exe` + `.msi`; gera `latest-windows-x86_64.json`
- **Linux** (`ubuntu-latest`): `npm run tauri:build` → `.AppImage` + `.deb`; gera `latest-linux-x86_64.json`
- Upload como artifact (30 dias)
- Tag `v*` → GitHub Release com instaladores
- Push em `master` → nightly release automático
