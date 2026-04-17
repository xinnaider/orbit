# Changelog Modal — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Exibir um modal com o histórico de novidades automaticamente quando o app é atualizado, e mantê-lo acessível via clique no badge de versão na sidebar.

**Architecture:** O `CHANGELOG.md` é importado em build time via Vite `?raw` e renderizado com o `Markdown.svelte` existente. A detecção de atualização compara `getAppVersion()` com `orbit.lastSeenVersion` no `localStorage`. O estado `showChangelog` vive em `App.svelte` e é passado para `Sidebar.svelte` via prop callback.

**Tech Stack:** SvelteKit 2 + Svelte 5, TypeScript, Vite `?raw` import, `marked` (via `Markdown.svelte` existente), `localStorage`.

---

## Mapa de arquivos

| Arquivo | Ação | Responsabilidade |
|---|---|---|
| `api/components/ChangelogModal.svelte` | Criar | Modal com overlay, header, badge de versão, corpo com Markdown |
| `api/App.svelte` | Modificar | Import do CHANGELOG, lógica de detecção de versão, render modal, prop para Sidebar |
| `api/components/Sidebar.svelte` | Modificar | Prop `onOpenChangelog`, badge de versão vira `<button>` |

---

## Task 1: Criar branch `feat/changelog-modal`

**Files:**
- (nenhum arquivo modificado nesta task)

- [ ] **Step 1: Criar e entrar na branch**

```bash
git checkout -b feat/changelog-modal
```

Expected: `Switched to a new branch 'feat/changelog-modal'`

---

## Task 2: Criar `ChangelogModal.svelte`

**Files:**
- Create: `api/components/ChangelogModal.svelte`

- [ ] **Step 1: Criar o componente**

Criar `api/components/ChangelogModal.svelte` com o seguinte conteúdo:

```svelte
<script lang="ts">
  import Markdown from './Markdown.svelte';

  export let changelogContent: string;
  export let currentVersion: string;
  export let onClose: () => void;

  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }
</script>

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  on:click={handleOverlayClick}
  on:keydown={(e) => e.key === 'Escape' && onClose()}
>
  <div class="modal">
    <div class="modal-header">
      <div class="modal-title">
        <span class="title-text">novidades do orbit</span>
        <span class="version-badge">v{currentVersion}</span>
      </div>
      <button class="close-btn" on:click={onClose} aria-label="Fechar">✕</button>
    </div>
    <div class="modal-body">
      <div class="current-badge">● versão atual — v{currentVersion}</div>
      <Markdown content={changelogContent} />
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    z-index: 600;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .modal {
    width: 480px;
    max-height: 520px;
    background: var(--bg2);
    border: 1px solid var(--bd2);
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
  }
  .modal-title {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title-text {
    font-size: var(--md);
    color: var(--t0);
    font-weight: 500;
  }
  .version-badge {
    font-size: 10px;
    color: var(--ac);
    background: var(--ac-d);
    border: 1px solid rgba(0, 212, 126, 0.2);
    border-radius: 3px;
    padding: 2px 7px;
  }
  .close-btn {
    background: none;
    border: none;
    color: var(--t2);
    font-size: 13px;
    cursor: pointer;
    padding: 2px 4px;
    line-height: 1;
    transition: color 0.15s;
  }
  .close-btn:hover {
    color: var(--t1);
  }
  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
  }
  .modal-body::-webkit-scrollbar {
    width: 4px;
  }
  .modal-body::-webkit-scrollbar-track {
    background: transparent;
  }
  .modal-body::-webkit-scrollbar-thumb {
    background: var(--bd2);
    border-radius: 2px;
  }
  .current-badge {
    font-size: 10px;
    color: var(--ac);
    margin-bottom: 14px;
  }
  /* Sobrescreve estilos do Markdown.svelte dentro do modal */
  .modal-body :global(h1) {
    display: none; /* esconde "# Changelog" do cabeçalho do arquivo */
  }
  .modal-body :global(h2) {
    font-size: 10px;
    color: var(--t2);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin: 16px 0 10px;
    font-weight: 500;
  }
  .modal-body :global(h3) {
    font-size: var(--sm);
    color: var(--t0);
    font-weight: 500;
    margin: 10px 0 4px;
  }
  .modal-body :global(p) {
    font-size: var(--sm);
    color: var(--t1);
    line-height: 1.6;
    margin-bottom: 8px;
  }
  .modal-body :global(hr) {
    border: none;
    border-top: 1px solid var(--bd);
    margin: 12px 0;
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add api/components/ChangelogModal.svelte
git commit -m "feat: add ChangelogModal component"
```

---

## Task 3: Modificar `App.svelte`

**Files:**
- Modify: `api/App.svelte`

O estado `showChangelog` e `appVersionForChangelog` vivem aqui. O `onMount` existente ganha a lógica de detecção. O template ganha o `ChangelogModal` e passa `onOpenChangelog` para `Sidebar`.

- [ ] **Step 1: Adicionar import do CHANGELOG e do componente**

Em `api/App.svelte`, adicionar no topo do bloco `<script>` (após os imports existentes):

```ts
  import changelogRaw from '../CHANGELOG.md?raw';
  import ChangelogModal from './components/ChangelogModal.svelte';
```

Também adicionar `getAppVersion` ao import existente de `./lib/tauri`:

```ts
  // Linha existente — adicionar getAppVersion:
  import {
    listSessions,
    checkClaude,
    onSessionCreated,
    onSessionOutput,
    onSessionState,
    onSessionStopped,
    onSessionRunning,
    onSessionError,
    onSessionRateLimit,
    getAppVersion,
  } from './lib/tauri';
```

- [ ] **Step 2: Declarar variáveis de estado**

Após a linha `let updateInterval: ReturnType<typeof setInterval> | null = null;` (última declaração de variável existente), adicionar:

```ts
  let showChangelog = false;
  let appVersionForChangelog = '';
```

- [ ] **Step 3: Adicionar lógica de detecção no `onMount`**

Dentro de `onMount`, logo após `const [existing, check] = await Promise.all([listSessions(), checkClaude()]);` (primeiras linhas do onMount), adicionar:

```ts
    const version = await getAppVersion();
    appVersionForChangelog = version;
    const lastSeen = localStorage.getItem('orbit.lastSeenVersion');
    if (lastSeen !== version) {
      showChangelog = true;
      localStorage.setItem('orbit.lastSeenVersion', version);
    }
```

- [ ] **Step 4: Renderizar `ChangelogModal` no template**

No template, logo após o bloco `{#if availableUpdate}...{/if}` e antes de `<div class="layout">`, adicionar:

```svelte
{#if showChangelog}
  <ChangelogModal
    changelogContent={changelogRaw}
    currentVersion={appVersionForChangelog}
    onClose={() => (showChangelog = false)}
  />
{/if}
```

- [ ] **Step 5: Passar prop `onOpenChangelog` para `Sidebar`**

No template, localizar `<Sidebar />` e modificar para:

```svelte
<Sidebar onOpenChangelog={() => (showChangelog = true)} />
```

- [ ] **Step 6: Verificar que `svelte-check` passa**

```bash
npm run lint
```

Expected: sem erros e `0 warnings`.

Se o TypeScript reclamar do import `?raw`, adicionar a seguinte declaração em `api/app.d.ts` (criar o arquivo se não existir):

```ts
declare module '*.md?raw' {
  const content: string;
  export default content;
}
```

- [ ] **Step 7: Commit**

```bash
git add api/App.svelte api/app.d.ts
git commit -m "feat: detect version change and show changelog modal on update"
```

---

## Task 4: Modificar `Sidebar.svelte`

**Files:**
- Modify: `api/components/Sidebar.svelte`

O badge de versão vira um `<button>` que chama `onOpenChangelog`. A prop tem valor padrão vazio para não quebrar uso existente.

- [ ] **Step 1: Adicionar prop `onOpenChangelog`**

Em `api/components/Sidebar.svelte`, dentro do bloco `<script>`, após as importações existentes e antes das declarações de variáveis (ex. antes de `let ctxMenu...`), adicionar:

```ts
  export let onOpenChangelog: () => void = () => {};
```

- [ ] **Step 2: Converter badge de versão em botão**

No template, localizar:

```svelte
      {#if appVersion}
        <span class="brand-version">v{appVersion}</span>
      {/if}
```

Substituir por:

```svelte
      {#if appVersion}
        <button class="brand-version" on:click={onOpenChangelog} title="ver novidades">
          v{appVersion}
        </button>
      {/if}
```

- [ ] **Step 3: Ajustar CSS de `.brand-version`**

No `<style>` de `Sidebar.svelte`, localizar o bloco `.brand-version` existente:

```css
  .brand-version {
    font-size: 10px;
    color: var(--t3);
    letter-spacing: 0.04em;
    margin-top: 1px;
  }
```

Substituir por:

```css
  .brand-version {
    font-size: 10px;
    color: var(--t3);
    letter-spacing: 0.04em;
    margin-top: 1px;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    transition: color 0.15s;
  }
  .brand-version:hover {
    color: var(--t1);
  }
```

- [ ] **Step 4: Commit**

```bash
git add api/components/Sidebar.svelte
git commit -m "feat: make version badge open changelog modal"
```

---

## Task 5: Atualizar `CHANGELOG.md` e fechar branch

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Adicionar entrada no CHANGELOG.md**

No topo da seção `## Abril 2026` em `CHANGELOG.md`, adicionar:

```markdown
### 07/04 · Novo — Histórico de novidades
O app agora exibe automaticamente o que mudou sempre que for atualizado para uma nova versão. O histórico completo fica acessível a qualquer momento clicando na versão exibida no canto superior da barra lateral.
```

- [ ] **Step 2: Commit final**

```bash
git add CHANGELOG.md
git commit -m "docs: add changelog modal entry to CHANGELOG"
```

---

## Verificação manual (sem testes automatizados)

O projeto não possui testes de componente Svelte. Verificar manualmente com `npm run dev:mock`:

1. **Auto-abertura:**
   - Abrir DevTools → Application → Local Storage → remover `orbit.lastSeenVersion`
   - Recarregar a página
   - Expected: modal abre automaticamente com badge "versão atual"

2. **Não abre na segunda vez:**
   - Fechar o modal e recarregar
   - Expected: modal não abre (localStorage já tem a versão correta)

3. **Abertura manual:**
   - Clicar no badge `v0.0.0` na sidebar
   - Expected: modal abre

4. **Fechar:**
   - Clicar no `✕` → modal fecha
   - Reabrir e clicar fora do modal (no overlay) → modal fecha

5. **Conteúdo:**
   - Verificar que o CHANGELOG.md está renderizado com headings visíveis
   - Verificar que `# Changelog` do cabeçalho do arquivo está oculto
