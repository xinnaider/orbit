# System Review — Spec

**Date:** 2026-05-24  
**Status:** Implemented  
**Scope:** E2E coverage, git workflow, chat/diffs, formatting, file reading, CI pipelines.

---

## Acceptance checklist

### Phase 0 — Foundation
- [x] Git commands registered in `lib.rs` (stage, commit, validate, per-file stage)
- [x] Git backend fixes (`git_diff_formatted`, `git_quick_commit`, numstat)
- [x] Mock IPC handlers for all git commands
- [x] CI: `e2e-mock` on PR, `cargo test` in test job

### Phase 1 — E2E
- [x] `e2e/helpers.ts` shared setup
- [x] Suites: chat, feed-diffs, git-panel, workspace, errors, smoke
- [x] `journal-fixtures.ts` deterministic tool output
- [x] Contract tests mock ↔ invoke commands
- [x] Tauri WebDriver scaffold (`e2e/tauri-smoke.spec.ts`, `test:e2e:tauri`)

### Phase 2 — Chat & Diffs
- [x] `PermissionDialog` wired in `CentralPanel`
- [x] `getDiff` / `getFileVersions` wrappers in `ui/lib/tauri/diff.ts`
- [x] Streaming tool diffs in `ToolCallEntry`
- [x] Removed unused `JournalEntry.svelte`, `MiniLog.svelte`

### Phase 3 — Formatting
- [x] Centralized `ui/lib/highlight.ts`
- [x] Markdown fenced blocks highlighted
- [x] Extended language support (Go, Java, C#, PHP, etc.)

### Phase 4 — Git UX
- [x] GitPanel toolbar (refresh, stage, commit, quick commit, reset)
- [x] `session:git-update` auto-refresh
- [x] Per-file stage/unstage
- [x] `gitDirty` badge in sidebar
- [x] Ahead/behind in header

### Phase 5 — File reading
- [x] `resolveFileDiff()` unified helper
- [x] Binary file detection in `git_diff_file`
- [x] `search_project_files` for @ picker fallback

### Phase 6 — Docs & CI
- [x] This spec
- [x] CHANGELOG entries
- [x] CLAUDE.md test scripts documented

---

## Running verification locally

```bash
npm test
npm run test:e2e
npm run test:rust
npm run lint
```

Tauri desktop smoke (requires built app + driver):

```bash
# Set TAURI_E2E=1 when tauri-driver is configured
npm run test:e2e:tauri
```
