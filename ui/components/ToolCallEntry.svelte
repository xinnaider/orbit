<script lang="ts">
  import type { JournalEntry } from '../lib/types';
  import { diffLines } from 'diff';
  import type { Change } from 'diff';
  import {
    FileText,
    FilePen,
    FilePlus,
    Terminal,
    Search,
    Folder,
    Bot,
    Wrench,
    Settings,
    Maximize2,
  } from 'lucide-svelte';
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
  export let streamingEntries: JournalEntry[] = [];

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

  $: rawChunks = hasEditDiff
    ? diffLines(entry.toolInput!.old_string as string, entry.toolInput!.new_string as string)
    : [];
  $: inlineLines = buildInlineLines(rawChunks);
  $: inlineOverflow = Math.max(0, inlineLines.length - 6);
  $: inlineVisible = inlineLines.slice(0, 6);
  $: modalLines = buildModalLines(rawChunks);

  $: writeLines = hasWriteContent
    ? (entry.toolInput!.content as string)
        .split('\n')
        .map((text, i) => ({ type: 'add' as const, text, lineNo: i + 1 }))
    : [];
  $: writeOverflow = Math.max(0, writeLines.length - 6);
  $: writeVisible = writeLines.slice(0, 6);

  // Code text (bash only — Write is handled via writeLines)
  $: codeText = hasBashCommand ? (entry.toolInput!.command as string) : '';

  const toolIconMap: Record<string, typeof FileText> = {
    read: FileText,
    edit: FilePen,
    write: FilePlus,
    bash: Terminal,
    grep: Search,
    glob: Folder,
    agent: Bot,
    skill: Wrench,
  };

  $: ToolIcon = toolIconMap[toolClass] ?? Settings;

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
    <span class="tool-icon {toolClass}"><ToolIcon size={13} /></span>
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
        title="Fullscreen"><Maximize2 size={11} /></button
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
                <span class="dl-prefix"
                  >{dl.type === 'add' ? '+' : dl.type === 'rem' ? '-' : ' '}</span
                >
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
          <pre class="code-inner code-text"><code>{@html doHighlight(codeText, 'bash')}</code></pre>
        {/if}

        {#if streamingEntries.length > 0 && !resultEntry}
          <div class="streaming-output">
            {#each streamingEntries as s}
              <pre class="streaming-line">{s.text}</pre>
            {/each}
          </div>
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
          <span class="tool-icon {toolClass}"><ToolIcon size={13} /></span>
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
                  <span class="dl-prefix"
                    >{dl.type === 'add' ? '+' : dl.type === 'rem' ? '-' : ' '}</span
                  >
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
    background: var(--bg3);
  }
  .tool-icon {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }
  .tool-icon.read,
  .tool-icon.grep,
  .tool-icon.glob {
    color: var(--user-fg);
  }
  .tool-icon.edit,
  .tool-icon.write {
    color: var(--tool-fg);
  }
  .tool-icon.bash {
    color: var(--ac);
  }
  .tool-icon.agent,
  .tool-icon.skill {
    color: var(--think-fg);
  }

  .tool {
    font-weight: 600;
    flex-shrink: 0;
  }
  .tool.read,
  .tool.grep,
  .tool.glob {
    color: var(--user-fg);
  }
  .tool.edit,
  .tool.write {
    color: var(--tool-fg);
  }
  .tool.bash {
    color: var(--ac);
  }
  .tool.agent {
    color: var(--think-fg);
  }
  .tool.skill {
    color: var(--think-fg);
  }
  .target {
    color: var(--t1);
    font-size: 12px;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .time {
    color: var(--t2);
    font-size: 10px;
    flex-shrink: 0;
  }
  .changes {
    display: flex;
    gap: 3px;
    flex-shrink: 0;
  }
  .added {
    color: var(--ac);
    font-size: 11px;
  }
  .removed {
    color: var(--s-error);
    font-size: 11px;
  }
  .expand-btn {
    flex-shrink: 0;
    background: var(--bg3);
    border: 1px solid var(--bd1);
    color: var(--t1);
    font-size: 11px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    line-height: 1;
    transition: all 0.15s;
  }
  .expand-btn:hover {
    background: var(--bg4);
    color: var(--user-fg);
    border-color: var(--user-fg);
  }

  .detail {
    padding: 4px 10px 4px 10px;
  }

  .code-card {
    border: 1px solid var(--bd1);
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
    background: var(--bg2);
    white-space: pre-wrap;
    color: var(--t1);
  }
  .code-text code {
    font-family: inherit;
  }

  /* Streaming bash output (progress events before final result) */
  .streaming-output {
    background: var(--bg2);
    border-top: 1px solid var(--bd1);
  }
  .streaming-line {
    margin: 0;
    padding: 2px 10px;
    font-size: 10px;
    line-height: 1.5;
    white-space: pre-wrap;
    color: var(--t2);
    font-family: 'Cascadia Code', 'Fira Code', monospace;
  }

  /* Result output section */
  .result-divider {
    height: 1px;
    background: var(--bd1);
  }
  .result-output {
    background: var(--bg2);
  }
  .result-pre {
    margin: 0;
    padding: 8px 10px;
    font-size: 11px;
    line-height: 1.5;
    white-space: pre-wrap;
    color: var(--t1);
  }

  /* Read output with line numbers */
  .read-output {
    background: var(--bg2);
  }
  .read-table {
    border-collapse: collapse;
    width: 100%;
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    font-size: 11px;
    line-height: 1.6;
  }
  .read-table tr:hover {
    background: var(--bg3);
  }
  .line-num {
    padding: 0 8px 0 6px;
    text-align: right;
    color: var(--t2);
    user-select: none;
    white-space: nowrap;
    border-right: 1px solid var(--bd1);
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
  .diff-line.add .dl-prefix {
    color: var(--ac);
  }
  .diff-line.rem .dl-prefix {
    color: var(--s-error);
  }
  .diff-line.ctx .dl-prefix {
    color: var(--t3);
  }
  .dl-code {
    flex: 1;
    min-width: 0;
    white-space: pre;
  }
  .code-inner .dl-code {
    overflow: hidden;
    text-overflow: ellipsis;
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

  .diff-block.modal-code-scroll {
    overflow-x: auto;
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
    color: var(--t0);
  }
  .detail :global(.hljs-punctuation) {
    color: var(--t1);
  }

  /* Fullscreen modal */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
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
    background: var(--bg1);
    border: 1px solid var(--bd2);
    border-radius: 10px;
    width: 100%;
    max-width: 900px;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--bd1);
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
    color: var(--t1);
    font-size: 16px;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 6px;
  }
  .modal-close:hover {
    background: var(--bg3);
    color: var(--t0);
  }
  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
  }
  .modal-section-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--t1);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .modal-code-scroll {
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    font-size: 12px;
    line-height: 1.7;
    max-height: 60vh;
    overflow-x: auto;
    overflow-y: auto;
  }
</style>
