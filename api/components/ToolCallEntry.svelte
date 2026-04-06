<script lang="ts">
  import type { JournalEntry } from '../lib/types';
  import { diffLines } from 'diff';
  import type { Change } from 'diff';
  import hljs from 'highlight.js/lib/core';
  import javascript from 'highlight.js/lib/languages/javascript';
  import typescript from 'highlight.js/lib/languages/typescript';
  import python from 'highlight.js/lib/languages/python';
  import rust from 'highlight.js/lib/languages/rust';
  import css from 'highlight.js/lib/languages/css';
  import xml from 'highlight.js/lib/languages/xml';
  import json from 'highlight.js/lib/languages/json';
  import bashLang from 'highlight.js/lib/languages/bash';
  import yaml from 'highlight.js/lib/languages/yaml';
  import markdownLang from 'highlight.js/lib/languages/markdown';

  hljs.registerLanguage('javascript', javascript);
  hljs.registerLanguage('typescript', typescript);
  hljs.registerLanguage('python', python);
  hljs.registerLanguage('rust', rust);
  hljs.registerLanguage('css', css);
  hljs.registerLanguage('xml', xml);
  hljs.registerLanguage('html', xml);
  hljs.registerLanguage('json', json);
  hljs.registerLanguage('bash', bashLang);
  hljs.registerLanguage('shell', bashLang);
  hljs.registerLanguage('yaml', yaml);
  hljs.registerLanguage('markdown', markdownLang);
  hljs.registerLanguage('svelte', xml);

  type DiffLine = {
    type: 'add' | 'rem' | 'ctx';
    text: string;
    lineNo: number;
  };

  export let entry: JournalEntry;
  export let resultEntry: JournalEntry | null = null;

  let modalOpen = false;

  $: toolClass = (entry.tool ?? '').toLowerCase();
  $: target = extractTarget(entry);
  $: timeStr = entry.timestamp.slice(11, 16);
  $: hasEditDiff =
    toolClass === 'edit' && entry.toolInput?.old_string && entry.toolInput?.new_string;
  $: hasWriteContent = toolClass === 'write' && entry.toolInput?.content;
  $: hasBashCommand = toolClass === 'bash' && entry.toolInput?.command;
  $: isReadTool = toolClass === 'read';
  $: hasDetail = hasEditDiff || hasWriteContent || hasBashCommand;

  $: lang = detectLang(target);

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

  // Code text (bash only — Write is handled via writeLines)
  $: codeText = hasBashCommand ? (entry.toolInput!.command as string) : '';

  const toolIcons: Record<string, string> = {
    read: '📄',
    edit: '✏️',
    write: '📝',
    bash: '⚡',
    grep: '🔍',
    glob: '📁',
    agent: '🤖',
    skill: '🔧',
  };

  $: icon = toolIcons[toolClass] ?? '⚙️';

  function extractTarget(e: JournalEntry): string {
    if (!e.toolInput) return '';
    if (e.toolInput.file_path) return e.toolInput.file_path as string;
    if (e.toolInput.command) {
      const cmd = e.toolInput.command as string;
      return cmd.split('\n')[0];
    }
    if (e.toolInput.pattern) return e.toolInput.pattern as string;
    if (e.toolInput.description) return e.toolInput.description as string;
    return '';
  }

  function shortPath(p: string): string {
    const parts = p.replace(/\\/g, '/').split('/');
    return parts.length > 2 ? parts.slice(-2).join('/') : p;
  }

  function detectLang(filePath: string): string {
    const ext = filePath.split('.').pop()?.toLowerCase() ?? '';
    const map: Record<string, string> = {
      js: 'javascript',
      jsx: 'javascript',
      mjs: 'javascript',
      ts: 'typescript',
      tsx: 'typescript',
      py: 'python',
      rs: 'rust',
      css: 'css',
      html: 'html',
      svelte: 'svelte',
      vue: 'html',
      json: 'json',
      yaml: 'yaml',
      yml: 'yaml',
      sh: 'bash',
      bash: 'bash',
      zsh: 'bash',
      md: 'markdown',
      toml: 'yaml',
    };
    return map[ext] ?? '';
  }

  function doHighlight(code: string, language: string): string {
    if (language && hljs.getLanguage(language)) {
      return hljs.highlight(code, { language }).value;
    }
    return hljs.highlightAuto(code).value;
  }

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

  // Strip "  123→" line number prefixes from Read output
  function stripLineNumbers(text: string): { lineNums: string[]; code: string } {
    const lines = text.split('\n');
    const lineNums: string[] = [];
    const codeLines: string[] = [];
    for (const line of lines) {
      const match = line.match(/^(\s*\d+)→(.*)$/);
      if (match) {
        lineNums.push(match[1]);
        codeLines.push(match[2]);
      } else {
        lineNums.push('');
        codeLines.push(line);
      }
    }
    return { lineNums, code: codeLines.join('\n') };
  }
</script>

<div class="tool-wrap">
  <div class="tool-header">
    <span class="icon">{icon}</span>
    <span class="tool {toolClass}">{entry.tool}</span>
    <span class="target mono">{shortPath(target)}</span>
    <span class="time">{timeStr}</span>
    {#if entry.linesChanged}
      <span class="changes">
        <span class="added">+{entry.linesChanged.added}</span>
        <span class="removed">-{entry.linesChanged.removed}</span>
      </span>
    {/if}
    {#if hasDetail || resultEntry?.output}
      <button
        class="expand-btn"
        onclick={(e) => {
          e.stopPropagation();
          modalOpen = true;
        }}
        title="Fullscreen">⛶</button
      >
    {/if}
  </div>

  {#if hasDetail || resultEntry?.output}
    <div class="detail">
      <div class="code-card">
        {#if hasEditDiff}
          <div class="diff-block code-inner">
            {#each inlineVisible as dl}
              <div class="diff-line {dl.type}">
                <span class="dl-num">{dl.lineNo}</span>
                <span class="dl-prefix">{dl.type === 'add' ? '+' : dl.type === 'rem' ? '-' : ' '}</span>
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

        {#if resultEntry?.output}
          {#if hasDetail}
            <div class="result-divider"></div>
          {/if}
          {#if isReadTool}
            {@const parsed = stripLineNumbers(resultEntry.output)}
            <div class="code-inner read-output">
              <table class="read-table">
                <tbody>
                  {#each parsed.code.split('\n') as line, li}
                    <tr>
                      <td class="line-num">{parsed.lineNums[li] ?? ''}</td>
                      <td class="line-code">{@html doHighlight(line, lang)}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {:else}
            <div class="code-inner result-output">
              <pre class="result-pre mono">{resultEntry.output}</pre>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  {/if}
</div>

{#if modalOpen}
  <div
    class="modal-overlay"
    onclick={() => (modalOpen = false)}
    role="dialog"
    tabindex="-1"
    onkeydown={(e) => e.key === 'Escape' && (modalOpen = false)}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <div class="modal-header">
        <div class="modal-title">
          <span class="icon">{icon}</span>
          <span class="tool {toolClass}">{entry.tool}</span>
          <span class="target mono">{target}</span>
        </div>
        <button class="modal-close" onclick={() => (modalOpen = false)}>✕</button>
      </div>
      <div class="modal-body detail">
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

        {#if resultEntry?.output}
          <div class="modal-section-label">Output</div>
          <div class="code-card modal-card">
            {#if isReadTool}
              {@const parsed = stripLineNumbers(resultEntry.output)}
              <div class="modal-code-scroll read-output">
                <table class="read-table">
                  <tbody>
                    {#each parsed.code.split('\n') as line, li}
                      <tr>
                        <td class="line-num">{parsed.lineNums[li] ?? ''}</td>
                        <td class="line-code">{@html doHighlight(line, lang)}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {:else}
              <div class="modal-code-scroll result-output">
                <pre class="result-pre mono">{resultEntry.output}</pre>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .tool-wrap {
    margin: 2px 0;
  }
  .tool-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    border-radius: 6px;
    font-size: 12px;
    transition: background 0.15s;
  }
  .tool-header:hover {
    background: var(--bg-hover);
  }
  .icon {
    font-size: 12px;
    flex-shrink: 0;
  }
  .tool {
    font-weight: 600;
    flex-shrink: 0;
  }
  .tool.read,
  .tool.grep,
  .tool.glob {
    color: var(--blue);
  }
  .tool.edit,
  .tool.write {
    color: var(--orange);
  }
  .tool.bash {
    color: var(--green);
  }
  .tool.agent {
    color: var(--purple);
  }
  .tool.skill {
    color: var(--pink);
  }
  .target {
    color: var(--text-secondary);
    font-size: 12px;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .time {
    color: var(--text-dim);
    font-size: 10px;
    flex-shrink: 0;
  }
  .changes {
    display: flex;
    gap: 3px;
    flex-shrink: 0;
  }
  .added {
    color: var(--green);
    font-size: 11px;
  }
  .removed {
    color: var(--red);
    font-size: 11px;
  }
  .expand-btn {
    flex-shrink: 0;
    background: var(--bg-overlay);
    border: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 11px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    line-height: 1;
    transition: all 0.15s;
  }
  .expand-btn:hover {
    background: var(--bg-hover);
    color: var(--blue);
    border-color: var(--blue);
  }

  .detail {
    padding: 4px 10px 4px 10px;
  }

  .code-card {
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }
  .code-inner {
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    font-size: 11px;
    line-height: 1.6;
    max-height: 300px;
    overflow-y: auto;
  }
  .code-text {
    padding: 8px 10px;
    margin: 0;
    background: var(--bg-code);
    white-space: pre-wrap;
    color: var(--text-secondary);
  }
  .code-text code {
    font-family: inherit;
  }

  /* Result output section */
  .result-divider {
    height: 1px;
    background: var(--border);
  }
  .result-output {
    background: var(--bg-code);
  }
  .result-pre {
    margin: 0;
    padding: 8px 10px;
    font-size: 11px;
    line-height: 1.5;
    white-space: pre-wrap;
    color: var(--text-muted);
  }

  /* Read output with line numbers */
  .read-output {
    background: var(--bg-code);
  }
  .read-table {
    border-collapse: collapse;
    width: 100%;
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    font-size: 11px;
    line-height: 1.6;
  }
  .read-table tr:hover {
    background: var(--bg-hover);
  }
  .line-num {
    padding: 0 8px 0 6px;
    text-align: right;
    color: var(--text-dim);
    user-select: none;
    white-space: nowrap;
    border-right: 1px solid var(--border);
    opacity: 0.6;
    vertical-align: top;
  }
  .line-code {
    padding: 0 8px;
    white-space: pre-wrap;
    word-break: break-all;
  }

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

  /* highlight.js tokens */
  .detail :global(.hljs-keyword),
  .detail :global(.hljs-selector-tag),
  .detail :global(.hljs-built_in) {
    color: var(--hl-keyword, #c678dd);
  }

  .detail :global(.hljs-string),
  .detail :global(.hljs-attr),
  .detail :global(.hljs-addition) {
    color: var(--hl-string, #98c379);
  }

  .detail :global(.hljs-number),
  .detail :global(.hljs-literal) {
    color: var(--hl-number, #d19a66);
  }

  .detail :global(.hljs-comment),
  .detail :global(.hljs-quote) {
    color: var(--hl-comment, #5c6370);
    font-style: italic;
  }

  .detail :global(.hljs-function),
  .detail :global(.hljs-title) {
    color: var(--hl-function, #61afef);
  }

  .detail :global(.hljs-type),
  .detail :global(.hljs-title.class_) {
    color: var(--hl-type, #e5c07b);
  }

  .detail :global(.hljs-variable),
  .detail :global(.hljs-template-variable) {
    color: var(--hl-variable, #e06c75);
  }

  .detail :global(.hljs-meta),
  .detail :global(.hljs-selector-class) {
    color: var(--hl-meta, #61afef);
  }

  .detail :global(.hljs-tag) {
    color: var(--hl-tag, #e06c75);
  }
  .detail :global(.hljs-name) {
    color: var(--hl-tag, #e06c75);
  }
  .detail :global(.hljs-attribute) {
    color: var(--hl-attr, #d19a66);
  }

  .detail :global(.hljs-params) {
    color: var(--text-primary);
  }
  .detail :global(.hljs-punctuation) {
    color: var(--text-secondary);
  }

  /* Fullscreen modal */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    animation: fadeIn 0.15s ease-out;
  }
  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  .modal {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: 100%;
    max-height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .modal-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    min-width: 0;
  }
  .modal-title .target {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .modal-close {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 6px;
  }
  .modal-close:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .modal-section-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .modal-code-scroll {
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    font-size: 12px;
    line-height: 1.7;
    max-height: none;
    overflow-y: visible;
  }
</style>
