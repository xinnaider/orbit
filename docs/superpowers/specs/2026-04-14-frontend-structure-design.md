# Frontend Structure Refactor — Spec

## Objetivo

Refatorar o frontend do Orbit para ser modular como o backend, melhorar legibilidade sem mudar a estética monospace/terminal, e criar um sistema de design tokens consistente.

## Escopo

### 1. Sistema de Design Tokens

**Spacing scale** (substituir raw pixels):
```css
:root {
  --sp-1: 2px;   /* tight: borders, tiny gaps */
  --sp-2: 4px;   /* compact: icon padding */
  --sp-3: 6px;   /* small: button padding, list gaps */
  --sp-4: 8px;   /* base: standard padding */
  --sp-5: 12px;  /* medium: section gaps */
  --sp-6: 16px;  /* large: card padding */
  --sp-7: 20px;  /* spacious: modal padding */
  --sp-8: 32px;  /* wide: section spacing */
}
```

**Typography scale** (já existe, refinar):
- `--xs: 10px` — labels, hints
- `--sm: 11px` — secondary text, sidebar items
- `--md: 12px` — body text, inputs
- `--base: 13px` — primary content
- `--lg: 14px` — emphasis, headers

**Border radius** (padronizar):
```css
--radius-sm: 3px;  /* inputs, buttons */
--radius-md: 6px;  /* cards, modals */
--radius-lg: 10px; /* panels */
```

### 2. Componentes Compartilhados

Extrair padrões repetidos em componentes reutilizáveis:

| Componente | Substitui |
|-----------|-----------|
| `ui/components/shared/Button.svelte` | `.btn`, `.ghost`, `.primary` redefinidos em 5+ componentes |
| `ui/components/shared/Field.svelte` | `.field`, `.label`, `.input` redefinidos em modals/forms |
| `ui/components/shared/Modal.svelte` | `.overlay`, `.modal`, `.modal-header` repetidos em 3 modals |
| `ui/components/shared/Chip.svelte` | `.backend-chip`, `.sub-item` padrão similar |

### 3. Decomposição de Componentes Grandes

#### ToolCallEntry (820 → ~3 componentes)
- `ToolCallEntry.svelte` — wrapper que faz dispatch por tool type
- `BashOutput.svelte` — renderiza output de bash com syntax highlight
- `DiffDisplay.svelte` — renderiza diffs de arquivo

#### NewSessionModal (792 → ~3 componentes)
- `NewSessionModal.svelte` — form shell, submit, validation
- `ProviderSelector.svelte` — backend chips, sub-provider list, model picker
- `SshFields.svelte` — SSH toggle, host/user/password, diagnose

#### InputBar (576 → ~2 componentes)
- `InputBar.svelte` — text input, submit
- `SlashCommandPicker.svelte` — autocomplete dropdown, slash command list

#### Sidebar (487 → ~2 componentes)
- `Sidebar.svelte` — layout shell, new session button
- `SessionList.svelte` — session items, grouping, context menu

### 4. Reorganização do `lib/`

#### `tauri.ts` (325 linhas → módulos por domínio)
```
ui/lib/tauri/
  index.ts          — re-exports tudo
  invoke.ts         — invoke/listen helpers + mock detection
  sessions.ts       — createSession, listSessions, stopSession, sendMessage, etc.
  providers.ts      — getProviders, diagnoseProvider, checkEnvVar
  projects.ts       — createProject, listProjects
  journal.ts        — getSessionJournal, getSubagentJournal
  events.ts         — onSessionCreated, onSessionOutput, etc.
  system.ts         — checkClaude, diagnoseSpawn, getAppVersion, etc.
```

#### Tipos deduplicados
- `TokenUsage` e `MiniLogEntry` ficam apenas em `types.ts`
- `Session` em `stores/sessions.ts` importa de `types.ts`

### 5. Melhorias de Legibilidade (sem mudar estética)

- **Aumentar contraste** nos temas: `--t2` e `--t3` estão muito escuros no tema dark
- **Line-height** mais generoso no Feed (1.5 → 1.6)
- **Espaçamento entre entries** no Feed (gap de 8px → 12px)
- **Font-weight 500** em headings/labels (atualmente tudo 400)
- **Border-bottom** sutil entre sections em vez de background color change
- **Hover states** consistentes em todos botões/interativos

## Fora de Escopo

- Mudar framework (continua Svelte 5)
- Redesign completo (manter layout sidebar + central + meta)
- Mudar tipografia (continua JetBrains Mono)
- Novos componentes de feature (só refatorar existentes)

## Critérios de Aceitação

- Zero componente com mais de 400 linhas
- Zero CSS raw pixel fora de tokens (exceto 1px borders)
- Zero tipo duplicado entre stores e types
- `tauri.ts` decomposto em módulos
- Todos os 5 temas funcionam sem regressão visual
- `svelte-check`, `eslint`, `prettier` passam
