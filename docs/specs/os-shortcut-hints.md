# OS-aware shortcut hints — ORB-5

**Issue:** ORB-5
**Status:** Implemented

---

## Objetivo

Exibir hints de atalhos de teclado de acordo com o sistema operacional do usuário. No Windows e Linux, usar `Ctrl`; no macOS, usar `⌘` (Command).

---

## Comportamento esperado

| Contexto | macOS | Windows / Linux |
|----------|-------|-----------------|
| Toggle inspector | `⌘I` | `Ctrl+I` |
| Split pane (footer) | `⌘\ split` | `Ctrl+\ split` |
| Sidebar footer | `… • ⌘\ split • ⌘I inspect` | `… • Ctrl+\ split • Ctrl+I inspect` |
| Botão browse (pasta/chave) | `⌘` (legado) | `…` |
| Handler real (App) | `metaKey \|\| ctrlKey` | idem |

A detecção usa `navigator.platform` / `userAgent` no WebView do Tauri (mesmo padrão de `TerminalPanel.svelte`).

---

## Casos de borda

- **Mock / Vitest / SSR:** sem `navigator` → tratar como não-mac (`Ctrl`).
- **Linux no Mac hardware:** segue `navigator`, não hardware.
- **Web build:** mesma heurística do browser.
- **Submit com Enter no NewSessionModal:** `metaKey` sozinho não dispara em Windows; usar `metaKey || ctrlKey`.

---

## Critérios de aceitação

1. Em ambiente Windows (ou teste com `platform: Win32`), footer da sidebar e badge do inspector mostram `Ctrl+I` e `Ctrl+\`, nunca `⌘`.
2. Em ambiente macOS (ou teste com `platform: MacIntel`), continuam mostrando `⌘I` e `⌘\`.
3. `App.svelte` continua alternando o inspector com Ctrl+I no Windows (já funcionava).
4. Testes unitários cobrem formatação para mac e windows.

---

## Pontos de teste manual

1. **Windows:** abrir Orbit → sidebar footer deve mostrar `Ctrl+\` e `Ctrl+I`; badge “inspector hidden” deve mostrar `Ctrl+I`.
2. **Windows:** pressionar `Ctrl+I` → painel direito abre/fecha.
3. **macOS:** mesmos textos com `⌘`; `⌘I` alterna o inspector.
4. **New session modal:** `Ctrl+Enter` no prompt (Windows) envia o formulário quando preenchido.
5. **Browse:** botão ao lado do path em nova sessão mostra `…` no Windows (não `⌘`).
