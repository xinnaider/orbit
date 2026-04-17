# Spec: Changelog Modal

**Data:** 2026-04-07
**Branch alvo:** `feat/changelog-modal`

---

## Objetivo

Exibir automaticamente um modal com o histórico de novidades quando o app é atualizado. Após visualizado, o modal permanece acessível via clique no badge de versão na sidebar.

---

## Decisões de design

| Decisão | Escolha |
|---|---|
| Fonte do conteúdo | `CHANGELOG.md` importado via Vite `?raw` (build time) |
| Detecção de "versão nova" | `localStorage` — chave `orbit.lastSeenVersion` comparada com `getAppVersion()` |
| Acesso manual | Clique no badge `v0.2.3` na sidebar |
| Renderização | `Markdown.svelte` existente (usa `marked`) com CSS customizado no modal |
| Escopo do changelog | Exibe o arquivo completo; versão atual destacada no topo |

---

## Arquivos tocados

### 1. `api/components/ChangelogModal.svelte` — NOVO

Componente modal responsável por:
- Receber `changelogContent: string` (o CHANGELOG.md completo) e `currentVersion: string` como props
- Renderizar o conteúdo com `Markdown.svelte`
- Exibir um badge "versão atual — vX.Y.Z" fixo no topo do corpo
- Fechar ao clicar no `✕` ou fora do modal (overlay)
- Emitir evento `close` ao fechar

**Props:**
```ts
export let changelogContent: string;
export let currentVersion: string;
export let onClose: () => void;
```

**Layout:**
```
┌─────────────────────────────────────────┐
│ novidades do orbit        [v0.2.3]   [✕] │  ← header fixo
├─────────────────────────────────────────┤
│ ● versão atual                           │  ← badge fixo
│                                          │
│ ## Abril 2026                            │
│ ### 07/04 · Novo — Split panes           │  ← markdown renderizado
│ ...                                      │
│ ─────────────────────────────────────── │
│ ## Março 2026                            │
│ ...                                      │  ← histórico em opacidade reduzida
│                                          │
└─────────────────────────────────────────┘  ← scroll interno
```

- Largura: `480px`, altura máxima: `520px`, scroll interno no corpo
- Overlay semitransparente com `z-index: 500` (acima de banners existentes que usam até 499)
- Fechar ao clicar no overlay (fora do modal)

### 2. `api/App.svelte` — MODIFICADO

Adicionar na lógica de `onMount`, após `getAppVersion()` já chamado em `Sidebar.svelte`:

```ts
import changelogRaw from '../CHANGELOG.md?raw';  // App.svelte está em api/, CHANGELOG.md na raiz

let showChangelog = false;
let appVersionForChangelog = '';

// dentro de onMount — adicionar chamada a getAppVersion() (não existe hoje em App.svelte):
const version = await getAppVersion();
appVersionForChangelog = version;
const lastSeen = localStorage.getItem('orbit.lastSeenVersion');
if (lastSeen !== version) {
  showChangelog = true;
  localStorage.setItem('orbit.lastSeenVersion', version);
}
```

Renderizar o modal no template:
```svelte
{#if showChangelog}
  <ChangelogModal
    changelogContent={changelogRaw}
    currentVersion={appVersionForChangelog}
    onClose={() => (showChangelog = false)}
  />
{/if}
```

Passar callback para `Sidebar`:
```svelte
<Sidebar onOpenChangelog={() => (showChangelog = true)} />
```

### 3. `api/components/Sidebar.svelte` — MODIFICADO

Adicionar prop:
```ts
export let onOpenChangelog: () => void = () => {};
```

Transformar o badge de versão de texto em botão:
```svelte
<!-- Antes -->
<span class="brand-version">v{appVersion}</span>

<!-- Depois -->
<button class="brand-version" on:click={onOpenChangelog} title="ver novidades">
  v{appVersion}
</button>
```

Mínima mudança de CSS: adicionar `cursor: pointer` e `hover` discreto no estado já existente de `.brand-version`.

---

## Comportamento

### Auto-abertura após update
1. App monta → `getAppVersion()` retorna `"0.2.3"`
2. `localStorage.getItem('orbit.lastSeenVersion')` retorna `"0.2.1"` (ou `null` na primeira vez)
3. Versões diferentes → `showChangelog = true` + `localStorage.setItem(..., "0.2.3")`
4. Modal abre com badge "versão atual — v0.2.3" no topo

### Abertura manual
1. Usuário clica em `v0.2.3` na sidebar
2. `onOpenChangelog()` → `showChangelog = true`
3. Modal abre (sem alterar `localStorage`)

### Fechar
- Clique no `✕` no header
- Clique no overlay fora do modal
- Ambos: `showChangelog = false`

---

## Não está no escopo

- Parsing customizado de categorias com tags coloridas (pode ser feito numa iteração futura)
- Marcar entradas individuais como lidas
- Animação de entrada/saída do modal
- Filtro por versão ou categoria

---

## Branch e CHANGELOG

- Branch: `feat/changelog-modal`
- Adicionar entrada em `CHANGELOG.md` antes do commit

---

## Checklist de implementação

- [ ] Criar branch `feat/changelog-modal`
- [ ] Criar `ChangelogModal.svelte`
- [ ] Modificar `App.svelte` (import `?raw`, lógica de versão, render modal, prop para Sidebar)
- [ ] Modificar `Sidebar.svelte` (prop `onOpenChangelog`, badge como botão)
- [ ] Verificar que `CHANGELOG.md` está acessível via `?raw` no Vite config
- [ ] Testar abertura automática (limpar `localStorage` e reiniciar)
- [ ] Testar abertura manual (clicar no badge)
- [ ] Testar que não abre na segunda inicialização sem atualização
- [ ] Lint + svelte-check
- [ ] Atualizar `CHANGELOG.md`
