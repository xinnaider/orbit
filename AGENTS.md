# AGENTS.md

Este arquivo direciona agents ao guia principal do projeto.

👉 **[CLAUDE.md](./CLAUDE.md)** — contem toda a referencia do projeto: stack, estrutura, convencoes, testes, CI, provider system, MCP integration, e mais.

👉 **[git.ts](./ui/lib/tauri/git.ts)** — wrapper de git com funções para stage, commit e diff formatado.

👉 **[docs/dev-workflow.md](./docs/dev-workflow.md)** — dev sem encher disco (`tauri:dev`, `clean`, `dev:mock`).

## Git Workflow Improvements

### PowerShell 5.1 Compatibility
- **Problema**: `&&` não é suportado em PowerShell 5.1
- **Solução**: Converter `&&` para `; if ($?) { }` automaticamente
- **Função**: `gitStageAll()`, `gitCommit()`, `gitDiffFormatted()` no Tauri backend

### Fluxo de Uso
1. **Edit file** → `edit ui/lib/tauri/git.ts`
2. **Stage changes** → `gitStageAll(cwd)`
3. **View diff** → `gitDiffFormatted(cwd, 'ui/lib/tauri/git.ts')`
4. **Commit** → `gitCommit(cwd, message)`

### One-Click Actions Sugeridas
- `Stage All Changed` → `git stage all`
- `Stage All Staged` → `git reset staged`
- `Commit Staged` → `git commit -m "$message"`
- `Reset Changes` → `git reset working-tree`
- `Quick Commit` → `git quick commit` (auto-generate message)

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
