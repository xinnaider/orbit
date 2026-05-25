# Dev workflow — disk and compile time

## What uses disk space

| Path | Typical size | Safe to delete |
|------|----------------|----------------|
| `tauri/target/` | 2–8 GB (debug) | Yes — `npm run clean` |
| `tauri/binaries/` | copy of `.exe` (~50–150 MB) | Yes |
| `node_modules/` | ~500 MB | `npm run clean:all` then `npm ci` |
| `build/`, `.svelte-kit/` | tens of MB | Yes |

## Recommended commands

| Goal | Command |
|------|---------|
| Daily dev (one Rust compile) | `npm run tauri:dev` |
| Frontend only, no Rust | `npm run dev:mock` |
| Unit tests TS | `npm run test` |
| Unit tests Rust (lib only) | `npm run test:rust` |
| Free disk | `npm run clean` |
| Nuclear reset | `npm run clean:all` && `npm ci` |

## What changed (lean dev)

1. **`tauri:dev` no longer runs `cargo build` twice** — only `ensure-sidecar` (placeholder or hardlink), then `tauri dev` compiles once.
2. **No separate `orbit-mcp` binary** — MCP uses `orbit.exe --mcp-stdio` (half the Rust artifacts).
3. **Sidecar slot uses hardlink** when possible — no duplicate executable in `tauri/binaries/`.
4. **Smaller debug symbols** — `tauri/.cargo/config.toml` sets `debug = 1`.

## Optional: build cache outside the repo

Set once (PowerShell):

```powershell
$env:CARGO_TARGET_DIR = "$env:TEMP\orbit-cargo-target"
```

Or uncomment `target-dir` in `tauri/.cargo/config.toml`.

## CI vs local

CI caches `tauri/target` via `rust-cache`. Locally, avoid running `tauri:build` unless you need installers — it creates **release** artifacts in addition to debug.
