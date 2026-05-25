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
