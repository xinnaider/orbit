# Diff Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the naive diff rendering in `ToolCallEntry.svelte` with a real Myers unified diff that shows only changed lines inline (max 6, with overflow → modal) and the full diff with context in the modal.

**Architecture:** Add the `diff` npm package and rewrite the script logic and HTML template inside the single existing component `ToolCallEntry.svelte`. No new files are created; all changes are self-contained. Helper functions `buildInlineLines` and `buildModalLines` are added as plain TypeScript functions in the component's `<script>` block.

**Tech Stack:** Svelte 5, TypeScript, `diff` npm package (`diffLines`), `highlight.js` (already present)

---

## File Map

| File | Change |
|---|---|
| `package.json` | Add `diff` + `@types/diff` |
| `api/components/ToolCallEntry.svelte` | Script: add import, helpers, reactive vars. HTML: replace both diff loops. CSS: replace `.diff-prefix`/`.diff-code`, add new classes. |

---

### Task 1: Install `diff` package

**Files:**
- Modify: `package.json`

- [ ] **Step 1: Install the package**

Run from the project root (`C:\Users\fernandonepen\Documents\agent-dashboard-v2`):

```bash
npm install diff
npm install --save-dev @types/diff
```

Expected output ends with something like:
```
added 1 package, and audited N packages in Xs
```

- [ ] **Step 2: Verify it landed in package.json**

Open `package.json` and confirm `"diff"` appears in `dependencies` and `"@types/diff"` in `devDependencies`.

- [ ] **Step 3: Commit**

```bash
git add package.json package-lock.json
git commit -m "feat: add diff package for Myers diff algorithm"
```

---

### Task 2: Replace script logic in `ToolCallEntry.svelte`

**Files:**
- Modify: `api/components/ToolCallEntry.svelte` (lines 1–141)

This task replaces the naive reactive diff vars (lines 46–52) with:
- A `DiffLine` type
- Two helper functions: `buildInlineLines` and `buildModalLines`
- New reactive vars: `rawChunks`, `inlineLines`, `inlineOverflow`, `inlineVisible`, `modalLines`, `writeLines`, `writeOverflow`, `writeVisible`

- [ ] **Step 1: Add the import**

At the top of `<script lang="ts">`, after the existing `import type { JournalEntry }` line, add:

```typescript
import { diffLines } from 'diff';
import type { Change } from 'diff';
```

- [ ] **Step 2: Add the `DiffLine` type**

After the imports (before the `export let` props), add:

```typescript
type DiffLine = {
  type: 'add' | 'rem' | 'ctx';
  text: string;
  lineNo: number;
};
```

- [ ] **Step 3: Add helper functions**

Add these two functions anywhere in the `<script>` block after `doHighlight` (around line 122):

```typescript
function buildInlineLines(chunks: Change[]): DiffLine[] {
  const result: DiffLine[] = [];
  let oldLine = 1;
  let newLine = 1;
  for (const chunk of chunks) {
    const lines = chunk.value.split('\n');
    // diffLines includes a trailing empty string when value ends with \n — drop it
    if (lines[lines.length - 1] === '') lines.pop();
    if (chunk.added) {
      for (const text of lines) {
        result.push({ type: 'add', text, lineNo: newLine++ });
      }
    } else if (chunk.removed) {
      for (const text of lines) {
        result.push({ type: 'rem', text, lineNo: oldLine++ });
      }
    } else {
      // context: advance both counters but don't emit lines
      oldLine += lines.length;
      newLine += lines.length;
    }
  }
  return result;
}

function buildModalLines(chunks: Change[]): DiffLine[] {
  const result: DiffLine[] = [];
  let oldLine = 1;
  let newLine = 1;
  for (const chunk of chunks) {
    const lines = chunk.value.split('\n');
    if (lines[lines.length - 1] === '') lines.pop();
    if (chunk.added) {
      for (const text of lines) {
        result.push({ type: 'add', text, lineNo: newLine++ });
      }
    } else if (chunk.removed) {
      for (const text of lines) {
        result.push({ type: 'rem', text, lineNo: oldLine++ });
      }
    } else {
      for (const text of lines) {
        result.push({ type: 'ctx', text, lineNo: newLine });
        oldLine++;
        newLine++;
      }
    }
  }
  return result;
}
```

- [ ] **Step 4: Replace the naive reactive diff vars**

Find and **remove** these lines (current lines 46–52):

```typescript
  // Diff lines
  $: oldLines = hasEditDiff ? (entry.toolInput!.old_string as string).split('\n') : [];
  $: newLines = hasEditDiff ? (entry.toolInput!.new_string as string).split('\n') : [];
  $: allDiffLines = [
    ...oldLines.map((l) => ({ type: 'rem' as const, text: l })),
    ...newLines.map((l) => ({ type: 'add' as const, text: l })),
  ];
```

Replace with:

```typescript
  // Diff lines — real Myers algorithm
  $: rawChunks = hasEditDiff
    ? diffLines(
        entry.toolInput!.old_string as string,
        entry.toolInput!.new_string as string,
      )
    : [];
  $: inlineLines = buildInlineLines(rawChunks);
  $: inlineOverflow = Math.max(0, inlineLines.length - 6);
  $: inlineVisible = inlineLines.slice(0, 6);
  $: modalLines = buildModalLines(rawChunks);

  // Write / Create lines (all additions)
  $: writeLines = hasWriteContent
    ? (entry.toolInput!.content as string)
        .split('\n')
        .map((text, i) => ({ type: 'add' as const, text, lineNo: i + 1 }))
    : [];
  $: writeOverflow = Math.max(0, writeLines.length - 6);
  $: writeVisible = writeLines.slice(0, 6);
```

- [ ] **Step 5: Also remove the now-unused `codeText` reactive var for Write**

Find and remove (current lines 54–59):

```typescript
  // Code text
  $: codeText = hasBashCommand
    ? (entry.toolInput!.command as string)
    : hasWriteContent
      ? (entry.toolInput!.content as string)
      : '';
```

Replace with (keep only the bash command text since Write is now handled via `writeLines`):

```typescript
  // Code text (bash only — Write is handled via writeLines)
  $: codeText = hasBashCommand ? (entry.toolInput!.command as string) : '';
```

- [ ] **Step 6: Commit**

```bash
git add api/components/ToolCallEntry.svelte
git commit -m "feat(diff): add Myers diff helpers and reactive vars"
```

---

### Task 3: Replace HTML template

**Files:**
- Modify: `api/components/ToolCallEntry.svelte` (template section, lines 143–281)

#### 3a — Inline section (inside `.code-card`)

- [ ] **Step 1: Replace the inline Edit diff block**

Find this block (inside `.code-card`, the `{#if hasEditDiff}` branch):

```svelte
        {#if hasEditDiff}
          <div class="code-inner">
            {#each allDiffLines as dl}
              <div class="diff-line {dl.type}">
                <span class="diff-prefix">{dl.type === 'rem' ? '-' : '+'}</span>
                <span class="diff-code">{@html doHighlight(dl.text, lang)}</span>
              </div>
            {/each}
          </div>
        {:else if hasBashCommand || hasWriteContent}
          <pre class="code-inner code-text"><code
              >{@html doHighlight(codeText, hasBashCommand ? 'bash' : lang)}</code
            ></pre>
        {/if}
```

Replace with:

```svelte
        {#if hasEditDiff}
          <div class="diff-block code-inner">
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
          <div class="diff-block code-inner">
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
        {:else if hasBashCommand}
          <pre class="code-inner code-text"><code
              >{@html doHighlight(codeText, 'bash')}</code
            ></pre>
        {/if}
```

#### 3b — Modal section

- [ ] **Step 2: Replace the modal Edit diff block**

Find this block inside `.modal-body`:

```svelte
        {#if hasEditDiff}
          <div class="modal-section-label">Changes</div>
          <div class="code-card modal-card">
            <div class="modal-code-scroll">
              {#each allDiffLines as dl}
                <div class="diff-line {dl.type}">
                  <span class="diff-prefix">{dl.type === 'rem' ? '-' : '+'}</span>
                  <span class="diff-code">{@html doHighlight(dl.text, lang)}</span>
                </div>
              {/each}
            </div>
          </div>
        {:else if hasBashCommand || hasWriteContent}
          <div class="modal-section-label">{hasBashCommand ? 'Command' : 'Content'}</div>
          <div class="code-card modal-card">
            <pre class="modal-code-scroll code-text"><code
                >{@html doHighlight(codeText, hasBashCommand ? 'bash' : lang)}</code
              ></pre>
          </div>
        {/if}
```

Replace with:

```svelte
        {#if hasEditDiff}
          <div class="modal-section-label">Changes</div>
          <div class="code-card modal-card">
            <div class="diff-block modal-code-scroll">
              {#each modalLines as dl}
                <div class="diff-line {dl.type}">
                  <span class="dl-num">{dl.lineNo}</span>
                  <span class="dl-prefix">{dl.type === 'add' ? '+' : dl.type === 'rem' ? '-' : ' '}</span>
                  <span class="dl-code">{@html doHighlight(dl.text, lang)}</span>
                </div>
              {/each}
            </div>
          </div>
        {:else if hasWriteContent}
          <div class="modal-section-label">New File</div>
          <div class="code-card modal-card">
            <div class="diff-block modal-code-scroll">
              {#each writeLines as dl}
                <div class="diff-line add">
                  <span class="dl-num">{dl.lineNo}</span>
                  <span class="dl-prefix">+</span>
                  <span class="dl-code">{@html doHighlight(dl.text, lang)}</span>
                </div>
              {/each}
            </div>
          </div>
        {:else if hasBashCommand}
          <div class="modal-section-label">Command</div>
          <div class="code-card modal-card">
            <pre class="modal-code-scroll code-text"><code
                >{@html doHighlight(codeText, 'bash')}</code
              ></pre>
          </div>
        {/if}
```

- [ ] **Step 3: Commit**

```bash
git add api/components/ToolCallEntry.svelte
git commit -m "feat(diff): replace naive diff HTML with unified diff rendering"
```

---

### Task 4: Update CSS

**Files:**
- Modify: `api/components/ToolCallEntry.svelte` (`<style>` block, lines 444–472)

- [ ] **Step 1: Replace the old diff CSS block**

Find and remove this entire block (the "Diff lines" section):

```css
  /* Diff lines */
  .diff-line {
    display: flex;
    padding: 0 8px;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .diff-line.rem {
    background: color-mix(in srgb, var(--red) 10%, transparent);
  }
  .diff-line.add {
    background: color-mix(in srgb, var(--green) 10%, transparent);
  }
  .diff-prefix {
    flex-shrink: 0;
    width: 16px;
    user-select: none;
    opacity: 0.6;
  }
  .diff-line.rem .diff-prefix {
    color: var(--red);
  }
  .diff-line.add .diff-prefix {
    color: var(--green);
  }
  .diff-code {
    flex: 1;
    min-width: 0;
  }
```

Replace with:

```css
  /* Diff lines */
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
    background: rgba(0, 212, 126, 0.07);
    color: #6bffaa;
  }
  .diff-line.rem {
    background: rgba(224, 72, 72, 0.08);
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

- [ ] **Step 2: Verify the app builds**

```bash
npm run build
```

Expected: build completes with no TypeScript errors. If you see `Property 'diff' does not exist` or similar, ensure the `diff` import and `@types/diff` are in place from Task 1.

- [ ] **Step 3: Manual verification**

Run the Tauri dev server:

```bash
npm run tauri dev
```

Open a session that has Edit tool calls. Verify:
- [ ] Edit tool inline: shows only the changed lines, red for removed, green for added, with line numbers
- [ ] Edit tool inline: if > 6 changed lines, shows "▸ +N linhas · clique para ver tudo" button
- [ ] Clicking the overflow button OR the `⛶` expand button opens the modal
- [ ] Modal: shows full unified diff including unchanged context lines (grey)
- [ ] Write tool inline: shows first 6 lines in green, overflow button if file is longer
- [ ] Write tool modal: shows all lines in green with line numbers
- [ ] Bash tool: unchanged — still shows command as plain code block
- [ ] Read tool: unchanged — still shows output with line numbers table
- [ ] ESC or click outside closes the modal

- [ ] **Step 4: Final commit**

```bash
git add api/components/ToolCallEntry.svelte
git commit -m "feat(diff): VS Code-style diff CSS with line numbers and overflow"
```
