# Split Panes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a 2×2 split-pane layout to Orbit so users can view up to 4 Claude Code sessions simultaneously, with directional drag-and-drop from the sidebar to open and assign panes.

**Architecture:** A new `splitLayout` writable store (in `api/lib/stores/layout.ts`) owns all pane state — which slots are open, which session each holds, and which is focused. The existing `selectedSessionId` becomes a `derived` from `splitLayout`, preserving all current consumers unchanged. Two new Svelte components (`PaneGrid` and `Pane`) replace the single `CentralPanel` in `App.svelte`. No Rust changes.

**Tech Stack:** Svelte 5 stores (writable + derived), HTML5 Drag API, CSS Grid, Vitest for store tests.

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `api/lib/stores/layout.ts` | **Create** | `splitLayout` store + actions (`assignSession`, `openPane`, `closePane`, `setFocused`, `_reset`) |
| `api/lib/stores/layout.test.ts` | **Create** | Unit tests for store actions + derived `selectedSessionId` |
| `api/lib/stores/sessions.ts` | **Modify** | Change `selectedSessionId` from `writable` to `derived` from `splitLayout` |
| `api/components/Sidebar.svelte` | **Modify** | Add `draggable` to session items; replace `selectedSessionId.set` with `splitLayout.assignSession`; fix delete handler |
| `api/components/Pane.svelte` | **Create** | Single grid slot: zone detection, drop handler, close button, focus-on-click, renders `CentralPanel` or placeholder |
| `api/components/PaneGrid.svelte` | **Create** | CSS Grid container; computes column/row template from `$splitLayout.visible`; renders one `<Pane>` per visible slot |
| `api/App.svelte` | **Modify** | Replace `<CentralPanel>` + empty state with `<PaneGrid>`; replace `selectedSessionId.set` calls with `splitLayout.assignSession` |

---

## Task 1: Layout store

**Files:**
- Create: `api/lib/stores/layout.ts`
- Create: `api/lib/stores/layout.test.ts`

- [ ] **Step 1: Write the failing tests**

```typescript
// api/lib/stores/layout.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { createSplitLayout } from './layout';

describe('splitLayout store', () => {
  let layout: ReturnType<typeof createSplitLayout>;

  beforeEach(() => {
    layout = createSplitLayout(); // fresh instance per test — no singleton state bleed
  });

  describe('initial state', () => {
    it('has only tl in visible', () => {
      expect(get(layout).visible).toEqual(['tl']);
    });
    it('focuses tl by default', () => {
      expect(get(layout).focused).toBe('tl');
    });
    it('all panes start as null', () => {
      const { panes } = get(layout);
      expect(panes.tl).toBeNull();
      expect(panes.tr).toBeNull();
      expect(panes.bl).toBeNull();
      expect(panes.br).toBeNull();
    });
  });

  describe('assignSession', () => {
    it('assigns a session id to a pane', () => {
      layout.assignSession('tl', 5);
      expect(get(layout).panes.tl).toBe(5);
    });
    it('clears a pane when null is assigned', () => {
      layout.assignSession('tl', 5);
      layout.assignSession('tl', null);
      expect(get(layout).panes.tl).toBeNull();
    });
    it('does not affect other panes', () => {
      layout.assignSession('tl', 5);
      expect(get(layout).panes.tr).toBeNull();
    });
  });

  describe('openPane', () => {
    it('adds pane to visible and assigns the session', () => {
      layout.openPane('tr', 3);
      const state = get(layout);
      expect(state.visible).toContain('tr');
      expect(state.panes.tr).toBe(3);
    });
    it('only updates session when pane is already visible', () => {
      layout.openPane('tl', 7); // tl already in visible by default
      const state = get(layout);
      expect(state.panes.tl).toBe(7);
      expect(state.visible.filter((p) => p === 'tl')).toHaveLength(1);
    });
    it('preserves existing visible panes', () => {
      layout.openPane('tr', 3);
      expect(get(layout).visible).toContain('tl');
    });
  });

  describe('closePane', () => {
    it('removes pane from visible', () => {
      layout.openPane('tr', 3);
      layout.closePane('tr');
      expect(get(layout).visible).not.toContain('tr');
    });
    it('shifts focus to first visible pane when focused pane is closed', () => {
      layout.openPane('tr', 3);
      layout.setFocused('tr');
      layout.closePane('tr');
      expect(get(layout).focused).toBe('tl');
    });
    it('preserves focus when a non-focused pane is closed', () => {
      layout.openPane('tr', 3);
      // focused is still 'tl'
      layout.closePane('tr');
      expect(get(layout).focused).toBe('tl');
    });
    it('refuses to close the last visible pane', () => {
      layout.closePane('tl');
      expect(get(layout).visible).toHaveLength(1);
      expect(get(layout).visible[0]).toBe('tl');
    });
  });

  describe('setFocused', () => {
    it('updates the focused pane', () => {
      layout.openPane('tr', 3);
      layout.setFocused('tr');
      expect(get(layout).focused).toBe('tr');
    });
  });
});
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
npx vitest run api/lib/stores/layout.test.ts
```

Expected: `Cannot find module './layout'` or similar — the file doesn't exist yet.

- [ ] **Step 3: Implement the store**

```typescript
// api/lib/stores/layout.ts
import { writable } from 'svelte/store';

export type PaneId = 'tl' | 'tr' | 'bl' | 'br';

export interface SplitLayout {
  panes: Record<PaneId, number | null>;
  visible: PaneId[];
  focused: PaneId;
}

const defaultLayout: SplitLayout = {
  panes: { tl: null, tr: null, bl: null, br: null },
  visible: ['tl'],
  focused: 'tl',
};

export function createSplitLayout() {
  const { subscribe, update, set } = writable<SplitLayout>(defaultLayout);

  return {
    subscribe,
    assignSession(paneId: PaneId, sessionId: number | null) {
      update((l) => ({ ...l, panes: { ...l.panes, [paneId]: sessionId } }));
    },
    openPane(paneId: PaneId, sessionId: number | null) {
      update((l) => {
        if (l.visible.includes(paneId)) {
          return { ...l, panes: { ...l.panes, [paneId]: sessionId } };
        }
        return {
          ...l,
          panes: { ...l.panes, [paneId]: sessionId },
          visible: [...l.visible, paneId],
        };
      });
    },
    closePane(paneId: PaneId) {
      update((l) => {
        if (l.visible.length <= 1) return l;
        const visible = l.visible.filter((p) => p !== paneId);
        const focused = l.focused === paneId ? visible[0] : l.focused;
        return { ...l, visible, focused };
      });
    },
    setFocused(paneId: PaneId) {
      update((l) => ({ ...l, focused: paneId }));
    },
    /** Test utility: restores store to its initial state. */
    _reset() {
      set(defaultLayout);
    },
  };
}

export const splitLayout = createSplitLayout();
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
npx vitest run api/lib/stores/layout.test.ts
```

Expected: all tests pass with no errors.

- [ ] **Step 5: Commit**

```bash
git add api/lib/stores/layout.ts api/lib/stores/layout.test.ts
git commit -m "feat: add splitLayout store with pane actions"
```

---

## Task 2: Make `selectedSessionId` a derived store

**Files:**
- Modify: `api/lib/stores/sessions.ts` (one line change)
- Modify: `api/lib/stores/layout.test.ts` (add derived store tests)

- [ ] **Step 1: Add derived store tests to `layout.test.ts`**

Append to the bottom of `api/lib/stores/layout.test.ts`:

```typescript
// ── selectedSessionId derived ─────────────────────────────────
// These tests use the module-level singleton `splitLayout` because
// `selectedSessionId` is derived from it directly.

import { splitLayout } from './layout';
import { selectedSessionId } from './sessions';

describe('selectedSessionId derived store', () => {
  beforeEach(() => splitLayout._reset());

  it('returns null when focused pane has no session', () => {
    expect(get(selectedSessionId)).toBeNull();
  });

  it('reflects the session id in the focused pane', () => {
    splitLayout.assignSession('tl', 5);
    expect(get(selectedSessionId)).toBe(5);
  });

  it('updates when focus moves to a different pane', () => {
    splitLayout.openPane('tr', 7);
    splitLayout.setFocused('tr');
    expect(get(selectedSessionId)).toBe(7);
    splitLayout.setFocused('tl');
    expect(get(selectedSessionId)).toBeNull();
  });
});
```

- [ ] **Step 2: Run to confirm the new tests fail**

```bash
npx vitest run api/lib/stores/layout.test.ts
```

Expected: the 3 `selectedSessionId derived store` tests fail because `selectedSessionId` is still a writable.

- [ ] **Step 3: Change `selectedSessionId` to derived in `sessions.ts`**

Open `api/lib/stores/sessions.ts`. Make these two changes:

**Change the import line** (line 1):
```typescript
// Before:
import { writable } from 'svelte/store';

// After:
import { writable, derived } from 'svelte/store';
```

**Change the `selectedSessionId` export** (the line that reads `export const selectedSessionId = writable<number | null>(null);`):
```typescript
// Before:
export const selectedSessionId = writable<number | null>(null);

// After:
import { splitLayout } from './layout';
export const selectedSessionId = derived(splitLayout, ($l) => $l.panes[$l.focused] ?? null);
```

> Note: place the `import { splitLayout }` line directly above `export const selectedSessionId` so the dependency is obvious. The rest of `sessions.ts` is unchanged — `getSelectedSession`, `upsertSession`, `updateSessionState` are all pure functions unaffected by this refactor.

- [ ] **Step 4: Run all store tests**

```bash
npx vitest run api/lib/stores/
```

Expected: all tests pass (layout.test.ts + sessions.test.ts).

- [ ] **Step 5: Commit**

```bash
git add api/lib/stores/sessions.ts api/lib/stores/layout.test.ts
git commit -m "refactor: make selectedSessionId a derived store from splitLayout"
```

---

## Task 3: Update Sidebar — fix click handler and add draggable

**Files:**
- Modify: `api/components/Sidebar.svelte`

> `selectedSessionId` is now read-only (derived). Every `selectedSessionId.set(...)` call in Sidebar must be replaced with the equivalent `splitLayout` action.

- [ ] **Step 1: Add the `splitLayout` import**

In `api/components/Sidebar.svelte`, find the existing import line:
```typescript
import { sessions, selectedSessionId, updateSessionState } from '../lib/stores/sessions';
```

Add a new import below it:
```typescript
import { splitLayout } from '../lib/stores/layout';
```

- [ ] **Step 2: Fix the session click handler**

Find the session `<button>` element (around line 150). Its `on:click` currently calls `selectedSessionId.set(s.id)`. Replace the entire `<button>` opening tag with:

```svelte
<button
  class="item"
  class:active
  draggable="true"
  on:dragstart={(e) => {
    e.dataTransfer!.setData('text/plain', String(s.id));
    e.dataTransfer!.effectAllowed = 'move';
  }}
  on:click={() => splitLayout.assignSession($splitLayout.focused, s.id)}
  on:contextmenu={(e) => onContextMenu(e, s)}
>
```

- [ ] **Step 3: Fix the delete handler**

Find the delete confirm button (around line 99-105). It currently has:
```javascript
if ($selectedSessionId === id) selectedSessionId.set(null);
```

Replace that single line with:
```javascript
for (const paneId of $splitLayout.visible) {
  if ($splitLayout.panes[paneId] === id) splitLayout.assignSession(paneId, null);
}
```

This clears the session from every pane that was showing the deleted session (not just the focused one).

- [ ] **Step 4: Run lint**

```bash
npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
npx eslint api/components/Sidebar.svelte --max-warnings 0
```

Expected: 0 errors, 0 warnings.

- [ ] **Step 5: Commit**

```bash
git add api/components/Sidebar.svelte
git commit -m "feat: make sidebar items draggable; use splitLayout for selection"
```

---

## Task 4: Create `Pane.svelte`

**Files:**
- Create: `api/components/Pane.svelte`

- [ ] **Step 1: Create the file**

```svelte
<!-- api/components/Pane.svelte -->
<script lang="ts">
  import { get } from 'svelte/store';
  import { sessions, getSelectedSession } from '../lib/stores/sessions';
  import { splitLayout, type PaneId } from '../lib/stores/layout';
  import CentralPanel from './CentralPanel.svelte';

  export let paneId: PaneId;

  // Maps each pane to its neighboring pane in each direction.
  // A missing key means no neighbor exists in that direction.
  const ADJACENT: Record<PaneId, Partial<Record<'right' | 'left' | 'bottom' | 'top', PaneId>>> = {
    tl: { right: 'tr', bottom: 'bl' },
    tr: { left: 'tl', bottom: 'br' },
    bl: { right: 'br', top: 'tl' },
    br: { left: 'bl', top: 'tr' },
  };

  // CSS grid-column / grid-row positions for each slot in the 2×2 grid.
  const GRID_POS: Record<PaneId, string> = {
    tl: 'grid-column: 1; grid-row: 1',
    tr: 'grid-column: 2; grid-row: 1',
    bl: 'grid-column: 1; grid-row: 2',
    br: 'grid-column: 2; grid-row: 2',
  };

  let container: HTMLDivElement;
  let dropZone: 'top' | 'bottom' | 'left' | 'right' | 'center' | null = null;

  $: sessionId = $splitLayout.panes[paneId];
  $: session = getSelectedSession($sessions, sessionId);
  $: focused = $splitLayout.focused === paneId;

  /**
   * Divide the pane into 5 zones based on cursor position relative to the element.
   * Top/bottom = 20% strips; left/right = 20% strips; everything else = center.
   */
  function getZone(e: DragEvent): 'top' | 'bottom' | 'left' | 'right' | 'center' {
    const rect = container.getBoundingClientRect();
    const x = (e.clientX - rect.left) / rect.width;
    const y = (e.clientY - rect.top) / rect.height;
    if (y < 0.2) return 'top';
    if (y > 0.8) return 'bottom';
    if (x < 0.2) return 'left';
    if (x > 0.8) return 'right';
    return 'center';
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    e.dataTransfer!.dropEffect = 'move';
    dropZone = getZone(e);
  }

  function onDragLeave() {
    dropZone = null;
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    dropZone = null;
    const raw = e.dataTransfer?.getData('text/plain');
    const sid = raw ? Number(raw) : null;
    if (!sid) return;

    const zone = getZone(e);
    const layout = get(splitLayout);

    // Center zone: always replace session in this pane.
    if (zone === 'center') {
      splitLayout.assignSession(paneId, sid);
      return;
    }

    // At 4 panes: border drops are silently ignored (spec §drag&drop).
    if (layout.visible.length >= 4) return;

    const adjacent = ADJACENT[paneId][zone];
    // No adjacent slot in this direction, or it's already open → treat as center.
    if (!adjacent || layout.visible.includes(adjacent)) {
      splitLayout.assignSession(paneId, sid);
      return;
    }

    splitLayout.openPane(adjacent, sid);
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
<div
  bind:this={container}
  class="pane"
  class:focused
  class:drop-top={dropZone === 'top'}
  class:drop-bottom={dropZone === 'bottom'}
  class:drop-left={dropZone === 'left'}
  class:drop-right={dropZone === 'right'}
  class:drop-center={dropZone === 'center'}
  style={GRID_POS[paneId]}
  on:click={() => splitLayout.setFocused(paneId)}
  on:dragover={onDragOver}
  on:dragleave={onDragLeave}
  on:drop={onDrop}
>
  {#if $splitLayout.visible.length > 1}
    <button
      class="close-btn"
      title="Close pane"
      on:click|stopPropagation={() => splitLayout.closePane(paneId)}
    >×</button>
  {/if}

  {#if session}
    <CentralPanel {session} />
  {:else}
    <div class="placeholder">+</div>
  {/if}
</div>

<style>
  .pane {
    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    border: 1px solid transparent;
    overflow: hidden;
  }

  .pane.focused {
    border-color: var(--bd2);
  }

  /* Drop-zone highlight: colored border on the side being targeted */
  .pane.drop-top    { border-top:    2px solid var(--ac); }
  .pane.drop-bottom { border-bottom: 2px solid var(--ac); }
  .pane.drop-left   { border-left:   2px solid var(--ac); }
  .pane.drop-right  { border-right:  2px solid var(--ac); }
  .pane.drop-center { border:        2px solid var(--ac); }

  .close-btn {
    position: absolute;
    top: 4px;
    right: 4px;
    z-index: 10;
    background: var(--bg2);
    border: 1px solid var(--bd);
    color: var(--t2);
    width: 18px;
    height: 18px;
    border-radius: 3px;
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .close-btn:hover {
    color: var(--fg);
    border-color: var(--ac);
  }

  .placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 24px;
    color: var(--bd2);
    cursor: default;
  }
</style>
```

- [ ] **Step 2: Run lint**

```bash
npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
npx eslint api/components/Pane.svelte --max-warnings 0
```

Expected: 0 errors, 0 warnings.

- [ ] **Step 3: Commit**

```bash
git add api/components/Pane.svelte
git commit -m "feat: add Pane component with directional drop-zone detection"
```

---

## Task 5: Create `PaneGrid.svelte`

**Files:**
- Create: `api/components/PaneGrid.svelte`

- [ ] **Step 1: Create the file**

```svelte
<!-- api/components/PaneGrid.svelte -->
<script lang="ts">
  import { splitLayout } from '../lib/stores/layout';
  import Pane from './Pane.svelte';

  // Whether the right column (col 2) needs to exist.
  // True when TR or BR is visible.
  $: hasTR = $splitLayout.visible.some((p) => p === 'tr' || p === 'br');

  // Whether the bottom row (row 2) needs to exist.
  // True when BL or BR is visible.
  $: hasBL = $splitLayout.visible.some((p) => p === 'bl' || p === 'br');

  $: cols = hasTR ? '1fr 1fr' : '1fr';
  $: rows = hasBL ? '1fr 1fr' : '1fr';
  $: gridStyle = `grid-template-columns: ${cols}; grid-template-rows: ${rows};`;
</script>

<div class="pane-grid" style={gridStyle}>
  {#each $splitLayout.visible as paneId (paneId)}
    <Pane {paneId} />
  {/each}
</div>

<style>
  .pane-grid {
    display: grid;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
</style>
```

- [ ] **Step 2: Run lint**

```bash
npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -5
npx eslint api/components/PaneGrid.svelte --max-warnings 0
```

Expected: 0 errors, 0 warnings.

- [ ] **Step 3: Commit**

```bash
git add api/components/PaneGrid.svelte
git commit -m "feat: add PaneGrid component with CSS grid layout"
```

---

## Task 6: Wire `App.svelte` — replace CentralPanel with PaneGrid

**Files:**
- Modify: `api/App.svelte`

- [ ] **Step 1: Update imports**

At the top of the `<script>` block in `api/App.svelte`, find:
```typescript
import CentralPanel from './components/CentralPanel.svelte';
```

Replace it with:
```typescript
import PaneGrid from './components/PaneGrid.svelte';
```

Add the `splitLayout` import alongside the existing store imports:
```typescript
import { splitLayout } from './lib/stores/layout';
```

> `selectedSessionId` import stays — it's still used as `$selectedSessionId` to drive `selected` and the MetaPanel condition.

- [ ] **Step 2: Fix auto-assign calls**

There are two `selectedSessionId.set(...)` calls in the `onMount` / event listener block. Replace both:

**First occurrence** (auto-select first session on load):
```typescript
// Before:
if (existing.length > 0 && !$selectedSessionId) selectedSessionId.set(existing[0].id);

// After:
if (existing.length > 0 && !$selectedSessionId) splitLayout.assignSession('tl', existing[0].id);
```

**Second occurrence** (auto-select newly created session when none is active):
```typescript
// Before:
if (!$selectedSessionId) selectedSessionId.set(s.id);

// After:
if (!$selectedSessionId) splitLayout.assignSession($splitLayout.focused, s.id);
```

- [ ] **Step 3: Replace the central area markup**

Find the layout `<div class="layout">` block and replace its inner content:

```svelte
<!-- Before: -->
<div class="layout">
  <Sidebar />
  {#if selected}
    <CentralPanel session={selected} />
  {:else}
    <div class="empty">
      {#if claudeCheck && !claudeCheck.found}
        <div class="claude-warn">
          <span class="warn-icon">⚠</span>
          <div>
            <div class="warn-title">claude CLI not found</div>
            <div class="warn-hint">
              {claudeCheck.hint ?? 'npm install -g @anthropic-ai/claude-code'}
            </div>
          </div>
        </div>
      {:else}
        <span class="empty-hint">no session selected</span>
      {/if}
    </div>
  {/if}
  {#if selected}
    <MetaPanel session={selected} />
  {/if}
</div>

<!-- After: -->
<div class="layout">
  <Sidebar />
  <div class="center">
    {#if claudeCheck && !claudeCheck.found}
      <div class="claude-warn">
        <span class="warn-icon">⚠</span>
        <div>
          <div class="warn-title">claude CLI not found</div>
          <div class="warn-hint">
            {claudeCheck.hint ?? 'npm install -g @anthropic-ai/claude-code'}
          </div>
        </div>
      </div>
    {/if}
    <PaneGrid />
  </div>
  {#if selected}
    <MetaPanel session={selected} />
  {/if}
</div>
```

- [ ] **Step 4: Update the `<style>` block**

Remove the `.empty` and `.empty-hint` rules (they no longer exist in the template). Add `.center`:

```css
.center {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
```

The `.claude-warn` and related rules (`.warn-icon`, `.warn-title`, `.warn-hint`) stay as-is.

- [ ] **Step 5: Run lint and tests**

```bash
npx svelte-check --tsconfig ./tsconfig.json 2>&1 | tail -10
npx eslint api/App.svelte --max-warnings 0
npx vitest run
```

Expected: 0 errors, 0 warnings, all tests pass.

- [ ] **Step 6: Commit**

```bash
git add api/App.svelte
git commit -m "feat: replace CentralPanel with PaneGrid in App layout"
```

---

## Task 7: CHANGELOG + final commit

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add changelog entry**

Open `CHANGELOG.md`. Under `## Abril 2026` (create the section if it doesn't exist), add:

```markdown
### 07/04 · Novo — Visualização em múltiplos painéis
O Orbit agora permite exibir até 4 sessões do Claude Code ao mesmo tempo.
Arraste uma sessão da barra lateral para a direita, baixo ou qualquer borda
do painel central para abrir um novo painel ao lado. O painel em foco determina
qual sessão é exibida no MetaPanel. Para fechar um painel, clique no × no canto superior direito.
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: update CHANGELOG for split panes feature"
```

---

## Self-review

**Spec coverage:**

| Spec requirement | Covered by |
|-----------------|-----------|
| Up to 4 panes, 2×2 grid | Task 5 (PaneGrid CSS grid) |
| Initial state: only TL visible | Task 1 (defaultLayout) |
| `splitLayout` store: panes, visible, focused | Task 1 |
| `selectedSessionId` derived from splitLayout | Task 2 |
| PaneGrid component | Task 5 |
| Pane component | Task 4 |
| App.svelte uses PaneGrid | Task 6 |
| Sidebar draggable items | Task 3 |
| dragstart sets `text/plain` sessionId | Task 3 |
| 5-zone detection (top/bottom/left/right/center 20%/80%) | Task 4 |
| Center drop: replace session in current pane | Task 4 |
| Border drop: open adjacent pane | Task 4 |
| Max 4 panes: border drops ignored | Task 4 |
| Adjacent already open: treat as center | Task 4 |
| CSS grid columns/rows from visible | Task 5 |
| Close button (visible.length > 1) | Task 4 |
| Close: remove from visible, keep session in store | Task 4 |
| Close focused pane: shift focus to first visible | Task 1 + Task 4 |
| Focused pane border highlight | Task 4 (.pane.focused) |
| MetaPanel follows focused pane | Task 6 ($selected derived from $selectedSessionId) |
| No Rust changes | ✓ no Rust files in file map |
| No CentralPanel/Feed/InputBar/MetaPanel internal changes | ✓ none in file map |

**Placeholder scan:** No TBD, TODO, or vague instructions found.

**Type consistency:**
- `PaneId` defined in `layout.ts`, imported in `Pane.svelte` and tests ✓
- `splitLayout.assignSession(paneId, sessionId)` — same signature across all callers ✓
- `splitLayout.openPane(adjacent, sid)` — `sid` is `number` from `Number(raw)` ✓
- `splitLayout.setFocused(paneId)` — called with `paneId: PaneId` prop ✓
- `createSplitLayout` exported for tests, `splitLayout` singleton exported for app ✓
