# Orbit Quiet Journal Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Orbit's approved Quiet Journal redesign: cleaner dark-first sidebar/topbar, timeline chat, hidden-by-default inspector, preserved split panes, preserved terminals, and existing Orbit icon.

**Architecture:** Keep existing Svelte stores and workspace tree. Change presentation in small component slices: preferences first, sidebar shell, pane header chrome, timeline feed, tool cards, composer, inspector visibility, terminal/pane integration, then verification. Prefer additive props/classes over broad store rewrites.

**Tech Stack:** Svelte 5, TypeScript, Vite, Vitest, @testing-library/svelte, Playwright smoke tests, existing CSS variable theme system.

---

## File Structure

### Create

- `ui/components/workspace/PanelHeader.component.test.ts` — component coverage for quiet pane header metadata, actions, drag, and close.
- `ui/lib/stores/preferences.test.ts` — coverage for default hidden inspector and persisted compact density preference.

### Modify

- `ui/lib/stores/preferences.ts` — change meta panel default to hidden and add compact-density store.
- `ui/App.svelte` — remove always-visible meta strip, add inspector reopen affordance and keyboard shortcut.
- `ui/components/Sidebar.svelte` — simplify into Quiet Console sidebar while preserving NewSessionModal, ThemePicker, context menu, and current Orbit SVG.
- `ui/components/Sidebar.component.test.ts` — update expectations for quiet sidebar copy, workspace grouping, and current Orbit icon render.
- `ui/components/workspace/PanelHeader.svelte` — style as pane header and support subtitle/meta/actions cleanly.
- `ui/components/workspace/PaneContainer.svelte` — add compact-pane class and keep terminal/chat/git panes inside same chrome language.
- `ui/components/workspace/SplitContainer.svelte` — make splitters quieter and preserve drag behavior.
- `ui/components/CentralPanel.svelte` — pass compact state to `Feed` and `InputBar`, simplify header metadata.
- `ui/components/Feed.svelte` — convert row rendering to timeline event rendering, with comfortable and compact density.
- `ui/components/Feed.component.test.ts` — update tests for timeline semantics and compact mode.
- `ui/components/ToolCallEntry.svelte` — convert tool display to quiet inline timeline card.
- `ui/components/ToolCallEntry.component.test.ts` — update tests for state labels and compact card.
- `ui/components/InputBar.svelte` — convert composer styling to quiet rounded surface and compact variant.
- `ui/components/InputBar.component.test.ts` — update tests for compact composer and controls.
- `ui/components/TerminalPanel.svelte` — adjust terminal surface spacing to fit pane header system.
- `ui/components/MetaPanel.svelte` — keep content, restyle as inspector surface opened on demand.
- `ui/app.css` — base typography/spacing variables for dark minimal direction.
- `ui/themes.css` — refine dark theme tokens first; preserve existing named themes.
- `e2e/orbit-smoke.spec.ts` — assert quiet sidebar and timeline feed still render after creating a session.

### Do not modify

- `ui/lib/assets/orbit.svg` — current Orbit icon must remain unchanged.
- Backend/Tauri session orchestration.
- Workspace store model unless implementation discovers an existing pane-type bug.

---

## Task 1: Preferences for Hidden Inspector and Compact Density

**Files:**
- Create: `ui/lib/stores/preferences.test.ts`
- Modify: `ui/lib/stores/preferences.ts`

- [ ] **Step 1: Write failing preference tests**

Create `ui/lib/stores/preferences.test.ts`:

```ts
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';

describe('preferences stores', () => {
  beforeEach(() => {
    vi.resetModules();
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
  });

  it('defaults inspector/meta panel to hidden for Quiet Journal', async () => {
    const { metaPanelVisible } = await import('./preferences');
    expect(get(metaPanelVisible)).toBe(false);
    metaPanelVisible.set(true);
    expect(localStorage.getItem('metaPanelVisible')).toBe('true');
  });

  it('persists compact density preference', async () => {
    const { compactDensity } = await import('./preferences');
    expect(get(compactDensity)).toBe(false);
    compactDensity.set(true);
    expect(get(compactDensity)).toBe(true);
    expect(localStorage.getItem('compactDensity')).toBe('true');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
npx vitest run --config vitest.config.js ui/lib/stores/preferences.test.ts
```

Expected: FAIL because `compactDensity` is not exported and `metaPanelVisible` currently defaults to `true`.

- [ ] **Step 3: Implement preference changes**

Modify `ui/lib/stores/preferences.ts`:

```ts
function createBooleanPreferenceStore(key: string, defaultValue: boolean) {
  const stored = typeof localStorage !== 'undefined' ? localStorage.getItem(key) : null;
  const initial = stored === null ? defaultValue : stored === 'true';
  const { subscribe, set } = writable<boolean>(initial);

  return {
    subscribe,
    set(value: boolean) {
      set(value);
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(key, String(value));
      }
    },
  };
}

export const theme = createThemeStore();
export const metaPanelVisible = createBooleanPreferenceStore('metaPanelVisible', false);
export const sidebarVisible = createBooleanPreferenceStore('sidebarVisible', true);
export const compactDensity = createBooleanPreferenceStore('compactDensity', false);
```

Remove the old `createMetaPanelVisibleStore` and `createSidebarVisibleStore` functions after adding the shared helper.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
npx vitest run --config vitest.config.js ui/lib/stores/preferences.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add ui/lib/stores/preferences.ts ui/lib/stores/preferences.test.ts
git commit -m "feat: default Orbit inspector to hidden"
```

---

## Task 2: Quiet Console Sidebar

**Files:**
- Modify: `ui/components/Sidebar.svelte`
- Modify: `ui/components/Sidebar.component.test.ts`

- [ ] **Step 1: Write failing sidebar tests**

Append these tests to `ui/components/Sidebar.component.test.ts`:

```ts
it('renders Quiet Console sidebar landmarks', () => {
  mockSessionsStore.set([
    makeSession({
      id: 1,
      name: 'Refactor billing flow',
      model: 'claude-sonnet-4-6',
      gitBranch: 'feature/auth',
      status: 'running',
      contextPercent: 41,
    }),
  ]);

  const { getByText, getByTestId, queryByText } = render(Sidebar);

  expect(getByTestId('orbit-brand-icon')).toBeTruthy();
  expect(getByText('orbit')).toBeTruthy();
  expect(getByText('Today')).toBeTruthy();
  expect(getByText('Refactor billing flow')).toBeTruthy();
  expect(getByText('feature/auth')).toBeTruthy();
  expect(getByText('41% ctx')).toBeTruthy();
  expect(queryByText('tokens')).toBeNull();
});

it('footer shows quiet workspace hints instead of only session count', () => {
  mockSessionsStore.set([makeSession({ id: 1, name: 'One' })]);
  const { getByText } = render(Sidebar);
  expect(getByText(/drag sessions into panes/i)).toBeTruthy();
  expect(getByText(/⌘I inspect/i)).toBeTruthy();
});
```

Update existing tests that assert `no sessions` and `3 sessions` so they expect the quiet empty/footer copy:

```ts
expect(getByText('No sessions yet')).toBeTruthy();
expect(getByText(/drag sessions into panes/i)).toBeTruthy();
```

- [ ] **Step 2: Run sidebar tests to verify failure**

Run:

```bash
npm run test:components -- ui/components/Sidebar.component.test.ts
```

Expected: FAIL because current sidebar does not expose `orbit-brand-icon`, `Today`, or quiet footer hints.

- [ ] **Step 3: Implement quiet sidebar markup**

In `ui/components/Sidebar.svelte`, keep existing modal/context-menu logic. Replace the main `<aside class="sidebar">` body with this structure, adapting existing event handlers and variables:

```svelte
<aside class="sidebar" data-testid="quiet-sidebar">
  <header class="header quiet-header">
    <div class="brand">
      <span class="brand-logo" data-testid="orbit-brand-icon">{@html OrbitLogo}</span>
      <span class="brand-name">orbit</span>
      {#if appVersion}
        <button class="brand-version" on:click={onOpenChangelog} title="What's new">
          v{appVersion}
        </button>
      {/if}
    </div>
    <div class="header-actions">
      <ThemePicker />
      <button
        type="button"
        class="new-btn quiet-new"
        aria-label="New session"
        data-testid="new-session-button"
        on:click={() => (showModal = true)}
      >new</button>
    </div>
  </header>

  <div class="quiet-search" aria-label="Search sessions">Search sessions…</div>

  <section class="session-section" aria-label="Today sessions">
    <div class="section-label">Today</div>
    <div class="session-list">
      {#if $sessions.length === 0}
        <div class="empty quiet-empty">No sessions yet</div>
      {:else}
        {#each $sessions.filter((s) => !s.parentSessionId) as s (s.id)}
          {@const hasChildren = getChildren($sessions, s.id).length > 0}
          {@const branchLabel = s.branchName ?? s.gitBranch ?? null}
          <button
            type="button"
            class="session-item quiet-session"
            class:active={$workspace.panes[$workspace.focusedPaneId ?? '']?.tabs.some((tab) => tab.target.kind === 'agent' && tab.target.sessionId === s.id)}
            draggable="true"
            data-testid="session-item"
            on:click={() => selectOrToggle(s, hasChildren)}
            on:contextmenu={(e) => onContextMenu(e, s)}
          >
            <span class="session-topline">
              <span class="session-title">{displayName(s)}</span>
              <span class="status-dot" style="background:{attentionColor(s.attention?.reason ?? null) || statusColor(s.status)}"></span>
            </span>
            <span class="session-subline">
              <span>{fmtModel(s.model)}</span>
              {#if branchLabel}<span>{branchLabel}</span>{/if}
              {#if (s.contextPercent ?? 0) > 0}<span>{Math.round(s.contextPercent ?? 0)}% ctx</span>{/if}
            </span>
          </button>
        {/each}
      {/if}
    </div>
  </section>

  <footer class="footer quiet-footer">drag sessions into panes • ⌘\ split • ⌘I inspect</footer>
</aside>
```

- [ ] **Step 4: Replace sidebar CSS with quiet styling**

Keep class names used by existing tests and add these styles near the sidebar style block:

```css
.sidebar {
  width: 282px;
  flex: 0 0 282px;
  background: var(--bg-sidebar, #0c0d0d);
  border-right: 1px solid var(--bd);
  padding: 18px 14px;
  gap: 16px;
}

.quiet-header { height: auto; padding: 0 6px 8px; border-bottom: 0; }
.brand { gap: 10px; }
.brand-logo { width: 28px; height: 28px; color: var(--ac); filter: drop-shadow(0 0 12px var(--ac-d)); }
.brand-logo :global(svg) { width: 26px; height: 26px; }
.quiet-new { width: auto; height: auto; padding: 6px 10px; border-radius: 999px; font-size: 12px; }
.quiet-search { height: 34px; display: flex; align-items: center; padding: 0 12px; border-radius: 12px; background: rgba(255,255,255,0.045); color: var(--t3); font-size: 13px; }
.section-label { padding: 0 8px; color: var(--t3); font-family: var(--mono); font-size: 10px; font-weight: 600; letter-spacing: 0.12em; text-transform: uppercase; }
.quiet-session { width: 100%; border: 1px solid transparent; border-radius: 15px; padding: 10px 11px; background: transparent; color: var(--t1); text-align: left; }
.quiet-session.active { color: var(--t0); background: rgba(255,255,255,0.06); border-color: rgba(255,255,255,0.075); }
.session-topline { display: flex; align-items: center; justify-content: space-between; gap: 10px; font-size: 13px; }
.session-title { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.session-subline { margin-top: 4px; display: flex; gap: 7px; color: var(--t3); font-family: var(--mono); font-size: 11px; white-space: nowrap; overflow: hidden; }
.status-dot { width: 7px; height: 7px; flex-shrink: 0; border-radius: 50%; box-shadow: 0 0 12px currentColor; }
.quiet-footer { margin-top: auto; padding: 10px 8px 0; border-top: 1px solid var(--bd); color: var(--t3); font-family: var(--mono); font-size: 11px; }
```

- [ ] **Step 5: Run sidebar tests**

Run:

```bash
npm run test:components -- ui/components/Sidebar.component.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add ui/components/Sidebar.svelte ui/components/Sidebar.component.test.ts
git commit -m "feat: simplify Orbit sidebar chrome"
```

---

## Task 3: Quiet Pane Header Chrome

**Files:**
- Create: `ui/components/workspace/PanelHeader.component.test.ts`
- Modify: `ui/components/workspace/PanelHeader.svelte`

- [ ] **Step 1: Write failing PanelHeader tests**

Create `ui/components/workspace/PanelHeader.component.test.ts`:

```ts
import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import PanelHeader from './PanelHeader.svelte';

describe('PanelHeader', () => {
  it('renders quiet pane header title, status, meta, actions, and close button', async () => {
    const onClose = vi.fn();
    const { getByText, getByLabelText, container } = render(PanelHeader, {
      props: {
        title: 'Refactor billing flow',
        status: 'running',
        closeLabel: 'Close chat pane',
        onClose,
        focused: true,
      },
      slots: {
        leading: '<span class="dot"></span>',
        meta: '<span class="quiet-pill">sonnet 4.6</span><span class="quiet-pill">41% ctx</span>',
        actions: '<button type="button">raw</button>',
      },
    });

    expect(container.querySelector('.panel-header.quiet-pane-header')).toBeTruthy();
    expect(getByText('Refactor billing flow')).toBeTruthy();
    expect(getByText('running')).toBeTruthy();
    expect(getByText('sonnet 4.6')).toBeTruthy();
    await fireEvent.click(getByLabelText('Close chat pane'));
    expect(onClose).toHaveBeenCalledOnce();
  });
});
```

- [ ] **Step 2: Run test to verify failure**

Run:

```bash
npm run test:components -- ui/components/workspace/PanelHeader.component.test.ts
```

Expected: FAIL because `.quiet-pane-header` is not present.

- [ ] **Step 3: Implement quiet pane header class and CSS**

In `PanelHeader.svelte`, change the header class:

```svelte
<header
  class="panel-header quiet-pane-header"
  class:focused
  draggable={!!dragPayload}
  on:dragstart={handleDragStart}
>
```

Update CSS values:

```css
.panel-header {
  display: flex;
  align-items: center;
  gap: 9px;
  height: 44px;
  padding: 0 14px;
  border-bottom: 1px solid var(--bd);
  background: color-mix(in srgb, var(--bg), white 1%);
  flex-shrink: 0;
  user-select: none;
  transition: opacity 0.15s, box-shadow 0.15s;
}

.panel-header.focused {
  box-shadow: inset 0 1px 0 color-mix(in srgb, var(--ac), transparent 84%);
}

.panel-title { font-size: 13px; font-weight: 700; font-family: var(--sans); }
.panel-subtitle { font-size: 11px; color: var(--t2); font-family: var(--mono); }
.panel-title-sep { display: none; }
.panel-meta :global(.quiet-pill),
.panel-actions :global(.quiet-pill) {
  border: 1px solid var(--bd1);
  border-radius: 999px;
  padding: 4px 8px;
  background: color-mix(in srgb, var(--bg2), transparent 28%);
  color: var(--t1);
  font-family: var(--mono);
  font-size: 10px;
}
.panel-icon-button { width: 24px; height: 24px; border-radius: 8px; background: color-mix(in srgb, var(--bg2), transparent 24%); }
```

- [ ] **Step 4: Run PanelHeader tests**

Run:

```bash
npm run test:components -- ui/components/workspace/PanelHeader.component.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add ui/components/workspace/PanelHeader.svelte ui/components/workspace/PanelHeader.component.test.ts
git commit -m "feat: add quiet pane header chrome"
```

---

## Task 4: Timeline Feed Structure and Compact Mode

**Files:**
- Modify: `ui/components/Feed.svelte`
- Modify: `ui/components/Feed.component.test.ts`

- [ ] **Step 1: Write failing timeline feed tests**

Add tests to `ui/components/Feed.component.test.ts`:

```ts
it('renders entries as Quiet Journal timeline events', () => {
  const entries = [
    makeEntry({ entryType: 'user', text: 'user prompt' }),
    makeEntry({ entryType: 'assistant', text: 'assistant response' }),
    makeEntry({ entryType: 'system', text: 'rate limit warning' }),
  ];
  const { container } = render(Feed, {
    props: { entries, status: '', provider: 'claude-code', cwd: null },
  });

  expect(container.querySelector('.timeline')).toBeTruthy();
  expect(container.querySelectorAll('.timeline-event').length).toBe(3);
  expect(container.querySelector('.timeline-event.user')).toBeTruthy();
  expect(container.querySelector('.timeline-event.assistant')).toBeTruthy();
  expect(container.querySelector('.timeline-event.system')).toBeTruthy();
});

it('supports compact timeline density', () => {
  const entries = [makeEntry({ entryType: 'user', text: 'compact prompt' })];
  const { container } = render(Feed, {
    props: { entries, status: '', provider: 'claude-code', cwd: null, compact: true },
  });
  expect(container.querySelector('.feed-scroller.compact')).toBeTruthy();
});

it('keeps working indicator inside timeline', () => {
  const { container, getByText } = render(Feed, {
    props: { entries: [], status: 'working', provider: 'claude-code', cwd: null },
  });
  expect(container.querySelector('.timeline-event.working')).toBeTruthy();
  expect(getByText('working')).toBeTruthy();
});
```

- [ ] **Step 2: Run Feed tests to verify failure**

Run:

```bash
npm run test:components -- ui/components/Feed.component.test.ts
```

Expected: FAIL because Feed uses `.row` rendering and does not accept `compact`.

- [ ] **Step 3: Add compact prop and actor helpers**

In `Feed.svelte` script section, add:

```ts
export let compact = false;

function eventClass(entry: JournalEntry): string {
  if (entry.entryType === 'toolCall') return 'tool';
  if (entry.entryType === 'toolResult' || entry.entryType === 'progress') return 'tool-detail';
  if (entry.entryType === 'assistant') return 'assistant';
  if (entry.entryType === 'user') return 'user';
  return 'system';
}

function actorLabel(entry: JournalEntry): string {
  if (entry.entryType === 'user') return 'you';
  if (entry.entryType === 'assistant') return `orbit / ${agentLabel}`;
  if (entry.entryType === 'toolCall') return 'tool';
  return entry.entryType;
}
```

- [ ] **Step 4: Replace Feed markup with timeline wrapper**

Replace the main `<div class="feed-scroller" ...>` content with this structure while preserving `hasMore`, `loadMore`, and scroll handlers:

```svelte
<div class="feed-scroller" class:compact bind:this={scrollerEl} onscroll={onScroll}>
  <div class="timeline">
    {#if hasMore}
      <button type="button" class="load-more" on:click={loadMore}>load earlier</button>
    {/if}

    {#each visibleItems as item, i (item.entry.seq ?? i)}
      {@const entry = item.entry}
      <article class={`timeline-event ${eventClass(entry)}`} aria-label={`${actorLabel(entry)} ${ts(entry)}`}>
        <div class="timeline-node" aria-hidden="true"></div>
        <div class="timeline-body">
          <div class="event-meta">
            <span class="event-actor">{actorLabel(entry)}</span>
            {#if ts(entry)}<span>{ts(entry)}</span>{/if}
          </div>

          {#if entry.entryType === 'toolCall'}
            <ToolCallEntry entry={entry} resultEntry={item.result} streamingEntries={item.streaming} {cwd} {compact} />
          {:else if entry.entryType === 'assistant'}
            <div class="event-text assistant-text">
              <Markdown text={entry.text ?? ''} />
            </div>
          {:else if entry.entryType === 'user'}
            <div class="event-text user-text">{entry.text}</div>
          {:else}
            <div class="event-text system-text">{entry.text}</div>
          {/if}
        </div>
      </article>
    {/each}

    {#if isWorking}
      <article class="timeline-event working" aria-label="agent working">
        <div class="timeline-node"></div>
        <div class="timeline-body">
          <div class="event-meta"><span class="event-actor">orbit / {agentLabel}</span></div>
          <div class="working-pill"><Sparkles size={12} /> working</div>
        </div>
      </article>
    {/if}
  </div>
</div>
```

- [ ] **Step 5: Add timeline CSS**

Replace row-heavy CSS in `Feed.svelte` with timeline styles. Keep existing scroll and load-more behavior.

```css
.feed-scroller { flex: 1; min-height: 0; overflow-y: auto; overflow-x: hidden; }
.timeline { width: min(900px, 100%); margin: 0 auto; padding: 28px 38px 22px; display: flex; flex-direction: column; gap: 13px; }
.timeline-event { display: grid; grid-template-columns: 20px 1fr; gap: 16px; position: relative; }
.timeline-event:not(:last-child)::before { content: ''; position: absolute; left: 9px; top: 29px; bottom: -14px; width: 1px; background: rgba(255,255,255,0.07); }
.timeline-node { width: 20px; height: 20px; border-radius: 50%; display: grid; place-items: center; margin-top: 4px; border: 1px solid var(--bd); background: var(--bg); }
.timeline-node::after { content: ''; width: 7px; height: 7px; border-radius: 50%; background: var(--t3); }
.timeline-event.user .timeline-node::after { background: var(--user-fg); box-shadow: 0 0 12px color-mix(in srgb, var(--user-fg), transparent 60%); }
.timeline-event.assistant .timeline-node::after,
.timeline-event.working .timeline-node::after { background: var(--ac); box-shadow: 0 0 12px var(--ac-border); }
.timeline-event.tool .timeline-node::after { background: var(--tool-fg); box-shadow: 0 0 12px color-mix(in srgb, var(--tool-fg), transparent 70%); }
.event-meta { display: flex; align-items: center; gap: 9px; margin-bottom: 5px; color: var(--t3); font-family: var(--mono); font-size: 11px; }
.event-actor { color: var(--t1); }
.event-text { color: var(--t0); font-size: 14px; line-height: 1.62; }
.user-text { max-width: 680px; width: fit-content; border: 1px solid var(--bd); border-radius: 18px; padding: 12px 14px; background: rgba(255,255,255,0.045); }
.system-text { color: var(--t1); font-family: var(--mono); font-size: 12px; }
.working-pill { display: inline-flex; align-items: center; gap: 8px; color: var(--think-fg); background: var(--think-bg); border: 1px solid color-mix(in srgb, var(--think-fg), transparent 82%); border-radius: 999px; padding: 7px 10px; font-family: var(--mono); font-size: 11px; }
.feed-scroller.compact .timeline { width: 100%; padding: 18px 22px; gap: 9px; }
.feed-scroller.compact .timeline-event { grid-template-columns: 16px 1fr; gap: 11px; }
.feed-scroller.compact .timeline-node { width: 16px; height: 16px; }
.feed-scroller.compact .timeline-node::after { width: 6px; height: 6px; }
.feed-scroller.compact .timeline-event:not(:last-child)::before { left: 7px; top: 24px; bottom: -10px; }
.feed-scroller.compact .event-text { font-size: 12px; line-height: 1.52; }
.feed-scroller.compact .user-text { border: 0; background: transparent; padding: 0; max-width: none; }
```

- [ ] **Step 6: Run Feed tests**

Run:

```bash
npm run test:components -- ui/components/Feed.component.test.ts
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add ui/components/Feed.svelte ui/components/Feed.component.test.ts
git commit -m "feat: render chat feed as Quiet Journal timeline"
```

---

## Task 5: Quiet Tool Call Cards

**Files:**
- Modify: `ui/components/ToolCallEntry.svelte`
- Modify: `ui/components/ToolCallEntry.component.test.ts`

- [ ] **Step 1: Write failing tool card tests**

Add tests to `ToolCallEntry.component.test.ts`:

```ts
it('renders quiet tool card with state label', () => {
  const entry = makeEntry({ entryType: 'toolCall', tool: 'bash', toolInput: { command: 'npm test' } });
  const result = makeEntry({ entryType: 'toolResult', output: '20 passed', exitCode: 0 });
  const { container, getByText } = render(ToolCallEntry, {
    props: { entry, resultEntry: result, streamingEntries: [], cwd: null },
  });

  expect(container.querySelector('.quiet-tool-card')).toBeTruthy();
  expect(getByText('bash')).toBeTruthy();
  expect(getByText('done')).toBeTruthy();
  expect(container.textContent).toContain('npm test');
  expect(container.textContent).toContain('20 passed');
});

it('renders compact quiet tool card', () => {
  const entry = makeEntry({ entryType: 'toolCall', tool: 'read', toolInput: { file_path: '/tmp/readme.md' } });
  const { container } = render(ToolCallEntry, {
    props: { entry, resultEntry: null, streamingEntries: [], cwd: null, compact: true },
  });
  expect(container.querySelector('.quiet-tool-card.compact')).toBeTruthy();
});
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
npm run test:components -- ui/components/ToolCallEntry.component.test.ts
```

Expected: FAIL because `.quiet-tool-card` and `compact` prop are absent.

- [ ] **Step 3: Add compact prop and status helper**

In `ToolCallEntry.svelte` script:

```ts
export let compact = false;

$: toolState = resultEntry ? (resultEntry.exitCode === 0 || resultEntry.exitCode == null ? 'done' : 'failed') : streamingEntries.length > 0 ? 'working' : 'queued';
```

- [ ] **Step 4: Wrap existing content in quiet card classes**

Keep existing tool-specific formatting helpers. Replace the outer card markup with:

```svelte
<div class="tc-card quiet-tool-card" class:compact>
  <div class="tc-head quiet-tool-head">
    <span class="tc-title">{entry.tool ?? 'tool'}</span>
    <span class="tc-state" class:failed={toolState === 'failed'}>{toolState}</span>
  </div>
  <div class="tc-body quiet-tool-body">
    <!-- keep existing command/file/input/result rendering here -->
  </div>
</div>
```

If the current component has separate root markup, preserve its content and move it inside `.quiet-tool-body`.

- [ ] **Step 5: Add quiet tool CSS**

In `ToolCallEntry.svelte` style block:

```css
.quiet-tool-card { border: 1px solid color-mix(in srgb, var(--tool-fg), transparent 84%); background: var(--tool-bg); border-radius: 16px; overflow: hidden; max-width: 720px; }
.quiet-tool-head { display: flex; justify-content: space-between; align-items: center; padding: 10px 12px; border-bottom: 1px solid color-mix(in srgb, var(--tool-fg), transparent 90%); color: var(--tool-fg); font-family: var(--mono); font-size: 12px; }
.tc-state { color: var(--t2); }
.tc-state.failed { color: var(--s-error); }
.quiet-tool-body { padding: 12px; background: rgba(0,0,0,0.12); color: var(--t1); font-family: var(--mono); font-size: 12px; line-height: 1.55; }
.quiet-tool-card.compact { border-radius: 13px; max-width: 100%; }
.quiet-tool-card.compact .quiet-tool-head { padding: 8px 10px; font-size: 11px; }
.quiet-tool-card.compact .quiet-tool-body { padding: 9px 10px; font-size: 11px; }
```

- [ ] **Step 6: Run tool tests**

Run:

```bash
npm run test:components -- ui/components/ToolCallEntry.component.test.ts
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add ui/components/ToolCallEntry.svelte ui/components/ToolCallEntry.component.test.ts
git commit -m "feat: restyle tool calls as quiet timeline cards"
```

---

## Task 6: Quiet Composer and Compact Composer

**Files:**
- Modify: `ui/components/InputBar.svelte`
- Modify: `ui/components/InputBar.component.test.ts`

- [ ] **Step 1: Write failing InputBar tests**

Add tests to `InputBar.component.test.ts`:

```ts
it('renders Quiet Journal composer controls', () => {
  const { container, getByText } = render(InputBar, {
    props: { sessionId: 1, cwd: 'C:/orbit', sessionStatus: 'running', provider: 'claude-code', providerModels: [] },
  });

  expect(container.querySelector('.quiet-composer')).toBeTruthy();
  expect(getByText('@ file')).toBeTruthy();
  expect(getByText('/ command')).toBeTruthy();
});

it('renders compact composer variant', () => {
  const { container } = render(InputBar, {
    props: { sessionId: 1, cwd: 'C:/orbit', sessionStatus: 'running', provider: 'claude-code', providerModels: [], compact: true },
  });

  expect(container.querySelector('.quiet-composer.compact')).toBeTruthy();
});
```

- [ ] **Step 2: Run InputBar tests to verify failure**

Run:

```bash
npm run test:components -- ui/components/InputBar.component.test.ts
```

Expected: FAIL because `compact` prop and quiet composer classes are absent.

- [ ] **Step 3: Add compact prop and visible control labels**

In `InputBar.svelte` script:

```ts
export let compact = false;
```

Ensure the rendered action labels include exact text `@ file` and `/ command` for tests and users.

- [ ] **Step 4: Add quiet composer class to root composer element**

Find the composer root around the textarea and use:

```svelte
<div class="input-wrap quiet-composer" class:compact>
```

For action buttons, use visible labels:

```svelte
<button type="button" class="composer-chip" on:click={...}>@ file</button>
<button type="button" class="composer-chip" on:click={...}>/ command</button>
```

Keep current send/stop behavior unchanged.

- [ ] **Step 5: Add quiet composer CSS**

In `InputBar.svelte` style block:

```css
.quiet-composer { width: min(840px, 100%); border: 1px solid var(--bd2); background: rgba(255,255,255,0.055); border-radius: 22px; padding: 12px; box-shadow: 0 24px 70px rgba(0,0,0,0.24); }
.quiet-composer textarea { font-size: 14px; line-height: 1.55; color: var(--t0); }
.composer-chip { border: 1px solid var(--bd1); color: var(--t1); background: rgba(255,255,255,0.03); border-radius: 999px; padding: 6px 9px; font-family: var(--mono); font-size: 11px; }
.quiet-composer.compact { min-height: 58px; border-radius: 16px; padding: 10px; }
.quiet-composer.compact textarea { font-size: 12px; }
.quiet-composer.compact .composer-chip { padding: 5px 8px; font-size: 10px; }
```

- [ ] **Step 6: Run InputBar tests**

Run:

```bash
npm run test:components -- ui/components/InputBar.component.test.ts
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add ui/components/InputBar.svelte ui/components/InputBar.component.test.ts
git commit -m "feat: add Quiet Journal composer"
```

---

## Task 7: Wire Compact Mode Through CentralPanel and Panes

**Files:**
- Modify: `ui/components/CentralPanel.svelte`
- Modify: `ui/components/workspace/PaneContainer.svelte`
- Modify: `ui/components/workspace/SplitContainer.svelte`

- [ ] **Step 1: Add failing compact wiring assertion to Feed test**

No new CentralPanel component test exists. Use an integration assertion in existing Feed/InputBar tests from Tasks 4 and 6 plus markup inspection in this task. Add this lightweight test to `Feed.component.test.ts` if not already present:

```ts
it('uses compact class only when compact prop is true', () => {
  const entries = [makeEntry({ entryType: 'assistant', text: 'normal' })];
  const normal = render(Feed, { props: { entries, status: '', provider: 'claude-code', cwd: null } });
  expect(normal.container.querySelector('.feed-scroller.compact')).toBeNull();
  cleanup();
  const compact = render(Feed, { props: { entries, status: '', provider: 'claude-code', cwd: null, compact: true } });
  expect(compact.container.querySelector('.feed-scroller.compact')).toBeTruthy();
});
```

- [ ] **Step 2: Run Feed tests**

Run:

```bash
npm run test:components -- ui/components/Feed.component.test.ts
```

Expected: PASS after Task 4. This step guards the prop before wiring it higher.

- [ ] **Step 3: Add compact class in PaneContainer**

In `PaneContainer.svelte`, derive a compact hint from the pane tab count and split context:

```ts
$: compactPane = pane ? pane.tabs.length > 1 || Object.keys($workspace.panes).length > 1 : false;
```

Change root class:

```svelte
<div class="pane-container" class:focused={isFocused} class:compact-pane={compactPane} ...>
```

Pass to `CentralPanel`:

```svelte
<CentralPanel
  {session}
  {paneId}
  focused={isFocused}
  compact={compactPane}
  onClose={canClose ? () => closePane(paneId) : null}
/>
```

- [ ] **Step 4: Accept compact prop in CentralPanel and pass down**

In `CentralPanel.svelte` script:

```ts
export let compact: boolean = false;
```

Pass to feed and input:

```svelte
<Feed bind:this={feedComponent} {entries} status={session.status} provider={session.provider} cwd={session.cwd} {compact} on:bottomchange={onFeedBottomChange} />
```

```svelte
<InputBar sessionId={session.id} cwd={session.cwd ?? ''} sessionStatus={session.status} provider={session.provider} providerModels={providerModelIds} {compact} />
```

- [ ] **Step 5: Make splitters quiet**

In `SplitContainer.svelte`, adjust splitter CSS to this visual language while preserving existing drag handlers:

```css
.splitter { background: transparent; position: relative; }
.splitter::after { content: ''; position: absolute; inset: 0; margin: auto; background: rgba(255,255,255,0.08); border-radius: 999px; }
.splitter.vertical::after { width: 1px; height: 100%; }
.splitter.horizontal::after { width: 100%; height: 1px; }
.splitter:hover::after { background: color-mix(in srgb, var(--ac), transparent 65%); }
```

Map class names to the actual splitter class names in the current component. Do not change drag math.

- [ ] **Step 6: Run component tests**

Run:

```bash
npm run test:components
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add ui/components/CentralPanel.svelte ui/components/workspace/PaneContainer.svelte ui/components/workspace/SplitContainer.svelte ui/components/Feed.component.test.ts
git commit -m "feat: enable compact Quiet Journal panes"
```

---

## Task 8: Hidden Inspector Integration

**Files:**
- Modify: `ui/App.svelte`
- Modify: `ui/components/MetaPanel.svelte`
- Modify: `ui/components/MetaPanel.component.test.ts`

- [ ] **Step 1: Write failing MetaPanel hidden-default test**

Add to `MetaPanel.component.test.ts`:

```ts
it('renders as quiet inspector surface', () => {
  const session = makeSession({ id: 1, name: 'Inspector Session', status: 'running' });
  const { container, getByText } = render(MetaPanel, { props: { session } });
  expect(container.querySelector('.meta.inspector')).toBeTruthy();
  expect(getByText('stats')).toBeTruthy();
});
```

- [ ] **Step 2: Run MetaPanel tests to verify failure**

Run:

```bash
npm run test:components -- ui/components/MetaPanel.component.test.ts
```

Expected: FAIL because `.inspector` class is missing.

- [ ] **Step 3: Restyle MetaPanel as inspector**

In `MetaPanel.svelte`, change root:

```svelte
<aside class="meta inspector" aria-label="Session inspector">
```

Add CSS:

```css
.inspector { width: 292px; border-left: 1px solid var(--bd); background: color-mix(in srgb, var(--bg), white 2%); padding: 14px; }
.inspector .tabs { height: 34px; border-bottom: 0; gap: 6px; }
.inspector .tab { border-radius: 999px; border: 1px solid var(--bd1); background: transparent; color: var(--t2); padding: 5px 9px; font-size: 11px; }
.inspector .tab.active { color: var(--ac); background: var(--ac-d2); border-color: var(--ac-border); }
.inspector .stat-group { border: 1px solid var(--bd); border-radius: 18px; background: rgba(255,255,255,0.025); padding: 14px; }
```

- [ ] **Step 4: Add keyboard shortcut and quieter reopen in App**

In `App.svelte`, import `metaPanelVisible` already exists. Add in script:

```ts
function onGlobalKeydown(event: KeyboardEvent) {
  const isInspect = (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'i';
  if (!isInspect) return;
  event.preventDefault();
  metaPanelVisible.set(!$metaPanelVisible);
}
```

Inside `onMount`, register and cleanup:

```ts
window.addEventListener('keydown', onGlobalKeydown);
return () => {
  window.removeEventListener('keydown', onGlobalKeydown);
  unlisteners.forEach((fn) => fn());
  if (updateInterval) clearInterval(updateInterval);
};
```

If `App.svelte` already has an `onDestroy`, place cleanup there instead:

```ts
onDestroy(() => {
  window.removeEventListener('keydown', onGlobalKeydown);
});
```

Change meta reopen button text/style:

```svelte
<button class="meta-reopen quiet-inspector-reopen" on:click={() => metaPanelVisible.set(true)} title="Show inspector">⌘I</button>
```

- [ ] **Step 5: Run MetaPanel tests**

Run:

```bash
npm run test:components -- ui/components/MetaPanel.component.test.ts
```

Expected: PASS.

- [ ] **Step 6: Run unit preference tests**

Run:

```bash
npx vitest run --config vitest.config.js ui/lib/stores/preferences.test.ts
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add ui/App.svelte ui/components/MetaPanel.svelte ui/components/MetaPanel.component.test.ts
git commit -m "feat: make session inspector hidden by default"
```

---

## Task 9: Terminal Pane Visual Integration

**Files:**
- Modify: `ui/components/TerminalPanel.svelte`
- Modify: `ui/components/workspace/PaneContainer.svelte`

- [ ] **Step 1: Inspect current terminal render manually**

Run mock/dev app:

```bash
npm run dev:mock
```

Expected: app starts on configured Vite port. Open Orbit, add terminal tab from pane add menu, observe current terminal spacing.

- [ ] **Step 2: Adjust TerminalPanel surface styles**

In `TerminalPanel.svelte`, preserve xterm initialization and Tauri calls. Adjust wrapper styles to match the mock:

```css
.terminal-panel { display: flex; flex-direction: column; flex: 1; min-width: 0; min-height: 0; background: var(--bg); }
.terminal-body { flex: 1; min-height: 0; padding: 14px 16px; background: #090a0a; }
.terminal-panel :global(.xterm) { font-family: var(--mono); }
```

If current class names differ, map these declarations to the existing root/body class names. Keep `terminal-panel` class because `ui/app.css` has xterm overrides targeting it.

- [ ] **Step 3: Ensure terminal header uses PanelHeader metadata**

Where `TerminalPanel` renders `PanelHeader`, ensure slots match the quiet pane header pattern:

```svelte
<PanelHeader title="Terminal" status={cwd} {focused} {onClose} closeLabel="Close terminal pane">
  <span slot="leading" class="dot terminal-dot"></span>
  <div slot="meta"><span class="quiet-pill">pwsh</span></div>
  <div slot="actions"><button class="terminal-action" type="button" title="New terminal">＋</button></div>
</PanelHeader>
```

Use the actual shell label if the component already knows it; otherwise use `terminal`.

- [ ] **Step 4: Run component tests**

Run:

```bash
npm run test:components
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add ui/components/TerminalPanel.svelte ui/components/workspace/PaneContainer.svelte
git commit -m "feat: align terminal panes with Quiet Journal chrome"
```

---

## Task 10: Dark Theme Tokens and Global Polish

**Files:**
- Modify: `ui/app.css`
- Modify: `ui/themes.css`

- [ ] **Step 1: Run current visual tests before styling**

Run:

```bash
npm run test:components
```

Expected: PASS before token changes.

- [ ] **Step 2: Update base typography variables**

In `ui/app.css`, keep imports working. Change base variables for readability:

```css
:root {
  --sans: 'Atkinson Hyperlegible', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --mono: 'JetBrains Mono', 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
  --xs: 10px;
  --sm: 11px;
  --md: 12px;
  --base: 13px;
  --lg: 14px;
  --lh: 1.6;
  --sp-1: 2px;
  --sp-2: 4px;
  --sp-3: 6px;
  --sp-4: 8px;
  --sp-5: 10px;
  --sp-6: 12px;
  --sp-7: 14px;
  --sp-8: 16px;
  --sp-9: 20px;
  --radius-sm: 6px;
  --radius-md: 12px;
  --radius-lg: 18px;
}
```

Update import to include Atkinson:

```css
@import url('https://fonts.googleapis.com/css2?family=Atkinson+Hyperlegible:wght@400;700&family=JetBrains+Mono:ital,wght@0,400;0,500;0,600;1,400&display=swap');
```

- [ ] **Step 3: Refine dark theme only**

In `ui/themes.css`, update dark theme values:

```css
:root,
[data-theme='dark'] {
  --bg: #090a0a;
  --bg1: #0c0d0d;
  --bg2: #101111;
  --bg3: #141515;
  --bg4: #191a1a;
  --bg-sidebar: #0c0d0d;
  --bd: rgba(241, 236, 227, 0.1);
  --bd1: rgba(241, 236, 227, 0.14);
  --bd2: rgba(241, 236, 227, 0.18);
  --t0: #f1ece3;
  --t1: #a19a90;
  --t2: #7a736b;
  --t3: #67615a;
  --ac: #7bd99d;
  --ac-d: rgba(123, 217, 157, 0.1);
  --ac-d2: rgba(123, 217, 157, 0.05);
  --ac-border: rgba(123, 217, 157, 0.22);
  --s-working: #7bd99d;
  --s-input: #d9b56c;
  --s-idle: #67615a;
  --s-error: #e47770;
  --s-init: #8dbbf7;
  --s-done: #3b3d3d;
  --user-fg: #8dbbf7;
  --user-bg: rgba(141, 187, 247, 0.07);
  --think-fg: #b9a1f4;
  --think-bg: rgba(185, 161, 244, 0.07);
  --tool-fg: #d9b56c;
  --tool-bg: rgba(217, 181, 108, 0.055);
  --result-fg: #7a736b;
  --result-bg: rgba(255, 255, 255, 0.025);
}
```

Do not alter light/nord/dracula/catppuccin/steel in this task.

- [ ] **Step 4: Run all JS/component tests**

Run:

```bash
npx vitest run --config vitest.config.js
npm run test:components
```

Expected: PASS for both commands.

- [ ] **Step 5: Commit**

```bash
git add ui/app.css ui/themes.css
git commit -m "style: refine dark Quiet Journal design tokens"
```

---

## Task 11: E2E Smoke Update for Quiet Journal

**Files:**
- Modify: `e2e/orbit-smoke.spec.ts`

- [ ] **Step 1: Update E2E assertions**

In `e2e/orbit-smoke.spec.ts`, after session creation and first message assertions, add:

```ts
await expect(page.getByTestId('quiet-sidebar')).toBeVisible();
await expect(page.locator('.timeline')).toBeVisible();
await expect(page.locator('.timeline-event').first()).toBeVisible();
await expect(page.getByTitle('Show inspector')).toBeVisible();
```

If the smoke currently creates a session but does not send a first message, keep the existing flow and assert the timeline container plus sidebar only.

- [ ] **Step 2: Run E2E locally**

Run:

```bash
npx playwright test --project=chromium e2e/orbit-smoke.spec.ts
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add e2e/orbit-smoke.spec.ts
git commit -m "test: assert Quiet Journal shell in E2E smoke"
```

---

## Task 12: Final Verification and Cleanup

**Files:**
- Modify only files required by failures discovered in this task.

- [ ] **Step 1: Run format and lint**

Run:

```bash
npx prettier --check "ui/**/*.{ts,svelte,css}"
npx svelte-kit sync && npx eslint ui --max-warnings 0 && npx svelte-check --tsconfig tsconfig.json --fail-on-warnings
```

Expected: PASS for both commands.

- [ ] **Step 2: Run unit tests**

Run:

```bash
npx vitest run --config vitest.config.js
```

Expected: PASS.

- [ ] **Step 3: Run component tests**

Run:

```bash
npm run test:components
```

Expected: PASS.

- [ ] **Step 4: Run E2E smoke**

Run:

```bash
npx playwright test --project=chromium e2e/
```

Expected: PASS.

- [ ] **Step 5: Manual visual check**

Run:

```bash
npm run dev:mock
```

Verify in browser:

- Orbit icon is unchanged.
- Sidebar is quieter and uses the current Orbit mark.
- Inspector is hidden by default.
- `⌘I` or `Ctrl+I` toggles inspector.
- Single chat uses comfortable timeline spacing.
- Split panes preserve multiple chats.
- Terminal can open as a pane.
- Narrow pane uses compact timeline styling.

- [ ] **Step 6: Commit any verification fixes**

If files changed during cleanup:

```bash
git status --short
git add <changed-files>
git commit -m "fix: polish Quiet Journal verification issues"
```

If no files changed, do not create an empty commit.

---

## Plan Self-Review

### Spec coverage

- Sidebar/topbar from A: Tasks 2 and 3.
- Chat timeline from B: Tasks 4 and 5.
- Multipanel behavior: Task 7.
- Terminal support: Task 9.
- Current Orbit icon unchanged: Task 2 explicitly uses existing `OrbitLogo`; file is not modified.
- Hidden inspector: Tasks 1 and 8.
- Dark minimal first: Task 10.
- Testing requirements: Tasks 1-12 include unit, component, E2E, and manual checks.

### Placeholder scan

The plan contains concrete files, test snippets, commands, and expected results. It avoids open-ended implementation gaps.

### Type consistency

New props are consistent across tasks:

- `compact: boolean` on `Feed`, `InputBar`, `ToolCallEntry`, and `CentralPanel`.
- `compactDensity` exported from `ui/lib/stores/preferences.ts`.
- `quiet-pane-header`, `quiet-sidebar`, `timeline`, `timeline-event`, and `quiet-tool-card` class names are used consistently in tests and implementation steps.
