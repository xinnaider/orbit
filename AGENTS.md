# AGENTS.md

Este arquivo direciona agents ao guia principal do projeto.

👉 **[CLAUDE.md](./CLAUDE.md)** — contem toda a referencia do projeto: stack, estrutura, convencoes, testes, CI, provider system, MCP integration, e mais.

## Cursor Cloud specific instructions

### Environment overview

Orbit is a Tauri 2 desktop app (Rust + SvelteKit). All standard dev commands are in `package.json` (see `CLAUDE.md` for the full table). No external services (Docker, databases, etc.) are required — SQLite is bundled via `rusqlite`.

### Rust toolchain

The `Cargo.toml` declares `rust-version = "1.85"` but some transitive dependencies require a newer Rust. Use `rustup default stable` (1.88+) rather than pinning to 1.85 exactly.

### Building the sidecar before clippy/build

`cargo clippy` and `cargo build` for the main Tauri app will fail if the `orbit-mcp` sidecar binary hasn't been built first (`binaries/orbit-mcp-<triple>` must exist). Run `node scripts/build-sidecar.mjs` (or `npm run build:sidecar`) before any `cargo clippy`/`cargo build` on the workspace. The `npm run tauri:dev` script does this automatically.

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
