# AGENTS.md

Este arquivo direciona agents ao guia principal do projeto.

👉 **[CLAUDE.md](./CLAUDE.md)** — contem toda a referencia do projeto: stack, estrutura, convencoes, testes, CI, provider system, MCP integration, e mais.

👉 **[git.ts](./ui/lib/tauri/git.ts)** — wrapper de git com funções para stage, commit e diff formatado.

👉 **[docs/dev-workflow.md](./docs/dev-workflow.md)** — dev sem encher disco (`tauri:dev`, `clean`, `dev:mock`).

## Regras de Git — Branches, Tracking e Push

### ⚠️ Criação de branch: SEMPRE sem tracking automático

**REGRA:** Ao criar uma nova branch, use `git checkout -b <nome>` sem especificar remote.
NUNCA use `git checkout -b <nome> <remote>/<branch>` — isso configura tracking automático.

**Por quê:** `git checkout -b fix/chat-feed origin/dev` configurou tracking automático para `origin/dev`. Isso fez o branch local parecer "conectado" ao remoto, causando confusão sobre para onde os commits iriam.

**Solução correta:**
```bash
# ✅ CORRETO — branch local pura, sem tracking
git checkout -b fix/chat-feed       # baseado no branch atual
git checkout -b fix/chat-feed --no-track origin/dev  # baseado no remote, sem tracking

# ❌ ERRADO — tracking automático configurado
git checkout -b fix/chat-feed origin/dev
```

**Verificação:** Após criar a branch, confirme que não há tracking:
```bash
git branch -vv
# ✅ Correto:   fix/chat-feed    42617d8 [mensagem do commit]
# ❌ Tracking:  fix/chat-feed    42617d8 [origin/dev] fix: suppress Windows...
#                                          ^^^^^^^^^^^
#                                          tracking ativo — NÃO QUEREMOS ISSO
```

Se o tracking foi configurado acidentalmente, remova com:
```bash
git branch --unset-upstream
```

### Push: só quando explicitamente solicitado

- **NUNCA** fazer push sem o usuário pedir explicitamente
- O primeiro push de uma nova branch deve usar o formato completo:
  ```bash
  git push -u origin <nome-da-branch>
  ```
  Isso empurra apenas para aquela branch remota, nunca para `dev`/`master`
- `git push` sem argumentos só funciona depois do `-u` acima — e empurra para a branch com **mesmo nome** no remoto, nunca para `dev`

### Commits ficam na branch atual

- Todo commit vai apenas para a branch em que você está
- Para confirmar antes de commitar:
  ```bash
  git branch --show-current   # mostra a branch atual
  ```
- Nenhum commit "vaza" para outra branch — a menos que você explicitamente faça merge, rebase ou cherry-pick

### Regressões: anotar em lessons.md

**SEMPRE** que uma regressão, erro de configuração ou confusão de workflow ocorrer:
1. Identifique o padrão do erro (não apenas o caso específico)
2. Adicione uma entrada em `docs/lessons.md` com:
   - **Regra**: o que fazer (ou não fazer)
   - **Por quê**: motivação / o que deu errado
   - **Quando aplicar**: contexto em que a regra vale
3. Itere nas lições existentes se o mesmo erro se repetir

### Fluxo de Uso — Git Stage/Commit (Tauri backend)
1. **Edit file** → `edit ui/lib/tauri/git.ts`
2. **Stage changes** → `gitStageAll(cwd)`
3. **View diff** → `gitDiffFormatted(cwd, 'ui/lib/tauri/git.ts')`
4. **Commit** → `gitCommit(cwd, message)`

## Cursor Cloud specific instructions

### Environment overview

Orbit is a Tauri 2 desktop app (Rust + SvelteKit). All standard dev commands are in `package.json` (see `CLAUDE.md` for the full table). No external services (Docker, databases, etc.) are required — SQLite is bundled via `rusqlite`.

### Rust toolchain

The `Cargo.toml` declares `rust-version = "1.85"` but some transitive dependencies require a newer Rust. Use `rustup default stable` (1.88+) rather than pinning to 1.85 exactly.

### Building the sidecar before clippy/build

`cargo clippy` and `cargo build` for the main Tauri app need the sidecar binary present (`binaries/orbit-mcp-<triple>`). Run `node scripts/ensure-sidecar.mjs` (or `npm run build:sidecar`) before `cargo clippy`/`cargo build`. `npm run tauri:dev` does this automatically via hardlink from the main executable (`--mcp-stdio`).

### Running the frontend without the Rust backend

Use `npm run dev:mock` to start the Vite dev server (port 1420) with mock Tauri IPC. This is useful for frontend-only development and testing in headless/cloud environments where the native Tauri window cannot open.

### svelte-check requires sync

Run `npx svelte-kit sync` before `npx svelte-check` if the `.svelte-kit/` directory doesn't exist yet. The `npm run check` script handles this automatically.

### Key commands

| Task | Command |
|------|---------|
| Install deps | `npm install` |
| Lint (all) | `npm run lint` |
| Frontend tests | `npm test` |
| Backend tests | `npm run test:rust` |
| Format check | `npm run format:check` |
| Dev (full) | `npm run tauri:dev` |
| Dev (mock) | `npm run dev:mock` |
