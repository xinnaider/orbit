# Diff Viewer — Design Spec

**Date:** 2026-04-06  
**Status:** Approved  
**Scope:** Enhance `api/components/ToolCallEntry.svelte` to render VS Code-style diffs for Edit, Write, and Create tool calls.

---

## Problem

The current diff rendering in `ToolCallEntry.svelte` is naive: it concatenates all `old_string` lines as `-` followed by all `new_string` lines as `+`. This produces an unreadable diff that bears no relation to the actual changes made.

---

## Goals

- Show a real Myers unified diff (correct line-level changes, properly intercalated)
- Inline preview in the feed: only changed lines, truncated at 6, themed to the app
- Modal: full diff with all context lines
- Write/Create tool: all lines displayed as additions (green), file treated as new
- No new components — change is contained to `ToolCallEntry.svelte`

---

## New Dependency

**`diff`** (npm) — small (~25KB, zero runtime deps), provides `diffLines()` using the Myers algorithm. Already used widely in editor tooling.

```
npm install diff
npm install --save-dev @types/diff
```

---

## Data Model

```typescript
import { diffLines } from 'diff';

type DiffLine = {
  type: 'add' | 'rem' | 'ctx';
  text: string;
  lineNo: number; // line number in the resulting file (new for add/ctx, old for rem)
};
```

### Edit tool

```typescript
$: rawChunks = hasEditDiff
  ? diffLines(entry.toolInput!.old_string as string,
              entry.toolInput!.new_string as string)
  : [];

// Inline: only changed lines, no context
$: inlineLines = buildInlineLines(rawChunks);   // DiffLine[], type add|rem only
$: inlineOverflow = Math.max(0, inlineLines.length - 6);
$: inlineVisible = inlineLines.slice(0, 6);

// Modal: all lines including context
$: modalLines = buildModalLines(rawChunks);     // DiffLine[], type add|rem|ctx
```

`buildInlineLines`: iterates chunks, skips context chunks, emits add/rem lines with correct line numbers tracked via a counter.

`buildModalLines`: same but also emits context chunks as `ctx` lines.

### Write / Create tool

```typescript
$: writeLines = hasWriteContent
  ? (entry.toolInput!.content as string)
      .split('\n')
      .map((text, i) => ({ type: 'add' as const, text, lineNo: i + 1 }))
  : [];
$: writeOverflow = Math.max(0, writeLines.length - 6);
$: writeVisible = writeLines.slice(0, 6);
```

---

## Inline Rendering (feed card)

Replaces the current `{#each allDiffLines as dl}` block inside `.code-inner`:

```svelte
{#if hasEditDiff}
  <div class="diff-block">
    {#each inlineVisible as dl}
      <div class="diff-line {dl.type}">
        <span class="dl-num">{dl.lineNo}</span>
        <span class="dl-prefix">{dl.type === 'add' ? '+' : '-'}</span>
        <span class="dl-code">{@html doHighlight(dl.text, lang)}</span>
      </div>
    {/each}
    {#if inlineOverflow > 0}
      <button class="diff-overflow" onclick={() => (modalOpen = true)}>
        ▸ +{inlineOverflow} linhas · clique para ver tudo
      </button>
    {/if}
  </div>
{:else if hasWriteContent}
  <div class="diff-block">
    {#each writeVisible as dl}
      <div class="diff-line add">
        <span class="dl-num">{dl.lineNo}</span>
        <span class="dl-prefix">+</span>
        <span class="dl-code">{@html doHighlight(dl.text, lang)}</span>
      </div>
    {/each}
    {#if writeOverflow > 0}
      <button class="diff-overflow" onclick={() => (modalOpen = true)}>
        ▸ +{writeOverflow} linhas · clique para ver tudo
      </button>
    {/if}
  </div>
{/if}
```

---

## Modal Rendering

Replaces the current `{#each allDiffLines as dl}` block inside `.modal-code-scroll`:

```svelte
{#each modalLines as dl}
  <div class="diff-line {dl.type}">
    <span class="dl-num">{dl.lineNo}</span>
    <span class="dl-prefix">
      {dl.type === 'add' ? '+' : dl.type === 'rem' ? '-' : ' '}
    </span>
    <span class="dl-code">{@html doHighlight(dl.text, lang)}</span>
  </div>
{/each}
```

Write/Create in modal: same as inline but uses all `writeLines` (no truncation).

---

## CSS (replaces existing diff classes in `ToolCallEntry.svelte` `<style>`)

The existing `.diff-prefix` and `.diff-code` rules will be removed. `.diff-line` keeps its name but gets new sub-elements. New classes introduced: `.diff-block`, `.dl-num`, `.dl-prefix`, `.dl-code`, `.diff-overflow`.

```css
.diff-block {
  font-family: var(--mono);
  font-size: 11px;
}

.diff-line {
  display: flex;
  gap: 6px;
  padding: 1px 8px;
  line-height: 1.5;
}

.diff-line.add {
  background: rgba(0, 212, 126, 0.07);   /* --ac with low opacity */
  color: #6bffaa;
}

.diff-line.rem {
  background: rgba(224, 72, 72, 0.08);   /* --s-error with low opacity */
  color: #ff8877;
}

.diff-line.ctx {
  color: var(--t3);
}

.dl-num {
  min-width: 28px;
  text-align: right;
  color: var(--t3);
  flex-shrink: 0;
  user-select: none;
}

.dl-prefix {
  flex-shrink: 0;
  width: 10px;
  user-select: none;
}

.diff-line.add .dl-prefix { color: var(--ac); }
.diff-line.rem .dl-prefix { color: var(--s-error); }
.diff-line.ctx .dl-prefix { color: var(--t3); }

.dl-code {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: pre;
}

.diff-overflow {
  display: block;
  width: 100%;
  background: none;
  border: none;
  border-top: 1px solid var(--bd);
  color: var(--t3);
  font-size: 10px;
  font-family: var(--mono);
  padding: 3px 8px;
  text-align: center;
  cursor: pointer;
}

.diff-overflow:hover {
  color: var(--t1);
  background: var(--bg3);
}
```

---

## Files Changed

| File | Change |
|---|---|
| `package.json` | Add `diff` + `@types/diff` |
| `api/components/ToolCallEntry.svelte` | Replace diff logic and rendering; add CSS |

No new files. No other components touched.

---

## Out of Scope

- Side-by-side (split) diff view
- Context lines in inline preview
- Monaco Editor integration
- Diff for Bash/Read tools (no before/after content available)
