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
    Copy,
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

  import { readFileContent } from '../lib/tauri/files';

  type DiffLine = {
    type: 'add' | 'rem' | 'ctx';
    text: string;
    lineNo: number;
  };

  export let entry: JournalEntry;
  export let resultEntry: JournalEntry | null = null;
  export let streamingEntries: JournalEntry[] = [];
  export let cwd: string | null = null;

  let modalOpen = false;
  let copied = false;

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  async function handleCopy() {
    copyToClipboard(await getCopyContent());
  }

  function resolveFilePath(filePath: string, cwd: string | null): string {
    if (!cwd || filePath.startsWith('/') || /^[A-Za-z]:/.test(filePath)) {
      return filePath;
    }
    const base = cwd.replace(/\\/g, '/').replace(/\/$/, '');
    const p = filePath.replace(/\\/g, '/').replace(/^\//, '');
    return `${base}/${p}`;
  }

  async function getCopyContent(): Promise<string> {
    const filePath = entry.toolInput?.file_path as string | undefined;
    if (filePath) {
      const resolved = resolveFilePath(filePath, cwd);
      try {
        return await readFileContent(resolved);
      } catch {
        // Fallback to inline content if file cannot be read
      }
    }
    if (hasEditDiff && entry.toolInput?.new_string) return entry.toolInput.new_string as string;
    if (hasWriteContent && entry.toolInput?.content) return entry.toolInput.content as string;
    if (hasBashCommand && entry.toolInput?.command) return entry.toolInput.command as string;
    if (resultEntry?.output) {
      if (isReadTool) return stripLineNumbers(resultEntry.output).code;
      return resultEntry.output;
    }
    return '';
  }

  function getBashOutputContent(): string {
    if (resultEntry?.output) return resultEntry.output;
    return '';
  }

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

<div class="tc-card">
  <div class="tc-header">
    <span class="tc-icon {toolClass}"><ToolIcon size={12} /></span>
    <span class="tc-title">
      <span class="tc-tool">{entry.tool}</span>
      {#if target}
        <span class="tc-sep">—</span>
        <span class="tc-target">{shortPath(target)}</span>
      {/if}
    </span>
    <span class="tc-spacer"></span>
    {#if entry.linesChanged}
      <span class="tc-changes">
        <span class="added">+{entry.linesChanged.added}</span>
        <span class="removed">-{entry.linesChanged.removed}</span>
      </span>
    {/if}
    <span class="tc-time">{timeStr}</span>
    {#if hasDetail || resultEntry?.output}
      {#if hasBashCommand && resultEntry?.output}
        <button
          class="tc-action tc-action--label"
          onclick={(e) => {
            e.stopPropagation();
            copyToClipboard(entry.toolInput!.command as string);
          }}
          title="Copiar comando"><Copy size={10} /> cmd</button
        >
        <button
          class="tc-action tc-action--label tc-action--darker"
          onclick={(e) => {
            e.stopPropagation();
            copyToClipboard(resultEntry.output!);
          }}
          title="Copiar output"><Copy size={10} /> out</button
        >
      {:else}
        <button
          class="tc-action"
          onclick={async (e) => {
            e.stopPropagation();
            copyToClipboard(await getCopyContent());
          }}
          title="Copiar conteúdo"><Copy size={10} /></button
        >
      {/if}
      <button
        class="tc-action"
        onclick={(e) => {
          e.stopPropagation();
          modalOpen = true;
        }}
        title="Fullscreen"><Maximize2 size={10} /></button
      >
    {/if}
  </div>

  {#if hasDetail || resultEntry?.output}
    <div class="tc-body">
      {#if hasEditDiff}
        <div class="diff-block">
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
      {:else if hasBashCommand}
        <div class="bash-body">
          <pre class="bash-code"><code>{@html doHighlight(codeText, 'bash')}</code></pre>
        </div>
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
          <div class="read-output">
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
          <div class="result-output">
            <pre class="result-pre mono">{resultEntry.output}</pre>
          </div>
        {/if}
      {/if}
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
          <div class="modal-card-section">
            <div class="modal-card-header">
              <span class="modal-card-label">Changes</span>
              <button
                class="card-copy-btn"
                onclick={async () => copyToClipboard(await getCopyContent())}
                title="Copiar conteúdo"
              >
                <Copy size={10} /> copy
              </button>
            </div>
            <div class="modal-card-body">
              <div class="diff-block">
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
          </div>
        {:else if hasWriteContent}
          <div class="modal-card-section">
            <div class="modal-card-header">
              <span class="modal-card-label">New File</span>
              <button
                class="card-copy-btn"
                onclick={async () => copyToClipboard(await getCopyContent())}
                title="Copiar conteúdo"
              >
                <Copy size={10} /> copy
              </button>
            </div>
            <div class="modal-card-body">
              <div class="diff-block">
                {#each writeLines as dl}
                  <div class="diff-line add">
                    <span class="dl-num">{dl.lineNo}</span>
                    <span class="dl-prefix">+</span>
                    <span class="dl-code">{@html doHighlight(dl.text, lang)}</span>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        {:else if hasBashCommand}
          <div class="modal-card-section">
            <div class="modal-card-header">
              <span class="modal-card-label">Command</span>
              <button
                class="card-copy-btn"
                onclick={() => copyToClipboard(entry.toolInput!.command as string)}
                title="Copiar comando"
              >
                <Copy size={10} /> copy
              </button>
            </div>
            <div class="modal-card-body">
              <pre class="code-text"><code>{@html doHighlight(codeText, 'bash')}</code></pre>
            </div>
          </div>
        {/if}

        {#if resultEntry?.output}
          <div class="modal-card-section">
            <div class="modal-card-header">
              <span class="modal-card-label">Output</span>
              <button
                class="card-copy-btn"
                onclick={() => copyToClipboard(resultEntry.output!)}
                title="Copiar output"
              >
                <Copy size={10} /> copy
              </button>
            </div>
            <div class="modal-card-body">
              {#if isReadTool}
                {@const parsed = stripLineNumbers(resultEntry.output)}
                <div class="read-output">
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
                <div class="result-output">
                  <pre class="result-pre mono">{resultEntry.output}</pre>
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  /* ── Tool Card ── */
  .tc-card {
    margin: var(--sp-2) 0;
    border: 1px solid var(--bd);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--bg1);
  }

  .tc-header {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-2) var(--sp-4);
    background: var(--bg2);
    border-bottom: 1px solid var(--bd);
    font-size: 10.5px;
    font-family: var(--mono);
    min-height: 28px;
  }

  .tc-icon {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }
  .tc-icon.read,
  .tc-icon.grep,
  .tc-icon.glob {
    color: var(--user-fg);
  }
  .tc-icon.edit,
  .tc-icon.write {
    color: var(--tool-fg);
  }
  .tc-icon.bash {
    color: var(--ac);
  }
  .tc-icon.agent,
  .tc-icon.skill {
    color: var(--think-fg);
  }

  .tc-title {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
  }
  .tc-tool {
    font-weight: 600;
    color: var(--t0);
    flex-shrink: 0;
  }
  .tc-sep {
    color: var(--t3);
    flex-shrink: 0;
  }
  .tc-target {
    color: var(--t1);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tc-spacer {
    flex: 1;
  }

  .tc-changes {
    display: flex;
    gap: var(--sp-1);
    flex-shrink: 0;
  }
  .added {
    color: var(--ac);
    font-size: 10px;
  }
  .removed {
    color: var(--s-error);
    font-size: 10px;
  }

  .tc-time {
    color: var(--t2);
    font-size: 9.5px;
    flex-shrink: 0;
  }

  .tc-action {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--t2);
    cursor: pointer;
    transition: all 0.12s;
    flex-shrink: 0;
  }
  .tc-action:hover {
    background: var(--bg3);
    color: var(--t0);
  }
  .tc-action--label {
    width: auto;
    gap: 3px;
    padding: 0 5px;
    font-size: 9px;
    font-family: var(--mono);
    font-weight: 500;
  }
  .tc-action--darker {
    background: var(--bg3);
    color: var(--t1);
  }
  .tc-action--darker:hover {
    background: var(--bg4);
    color: var(--t0);
  }

  /* ── Body ── */
  .tc-body {
    font-family: var(--mono);
    font-size: 10.5px;
    line-height: 1.55;
  }

  /* ── Diff block ── */
  .diff-block {
    font-family: var(--mono);
    line-height: 1.5;
  }
  .diff-line {
    display: flex;
    align-items: stretch;
    padding: 0 var(--sp-4);
    font-size: 10.5px;
    min-height: 20px;
  }
  .diff-line.ctx {
    background: var(--bg1);
  }
  .diff-line.add {
    background: var(--diff-add-bg);
  }
  .diff-line.rem {
    background: var(--diff-rem-bg);
  }

  .dl-num {
    width: 28px;
    flex-shrink: 0;
    color: var(--t3);
    text-align: right;
    padding-right: var(--sp-2);
    user-select: none;
    font-size: 10px;
  }
  .dl-prefix {
    width: 10px;
    flex-shrink: 0;
    user-select: none;
    color: var(--t3);
  }
  .dl-code {
    flex: 1;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .diff-overflow {
    width: 100%;
    text-align: left;
    padding: var(--sp-2) var(--sp-4);
    background: var(--bg2);
    border: none;
    border-top: 1px solid var(--bd);
    color: var(--t2);
    font-size: 10.5px;
    cursor: pointer;
    font-family: var(--mono);
  }
  .diff-overflow:hover {
    background: var(--bg3);
    color: var(--t0);
  }

  /* ── Bash body ── */
  .bash-body {
    padding: var(--sp-3) var(--sp-4);
  }
  .bash-code {
    margin: 0;
    font-family: var(--mono);
    font-size: 10.5px;
    line-height: 1.5;
    color: var(--ac);
  }
  .bash-code code {
    font-family: var(--mono);
  }

  /* ── Read output ── */
  .read-output {
    padding: var(--sp-3) var(--sp-4);
    overflow-x: auto;
  }
  .read-output table {
    font-family: var(--mono);
    font-size: 10.5px;
    border-collapse: collapse;
  }
  .read-output td {
    padding: 0 var(--sp-2);
    line-height: 1.55;
    vertical-align: top;
  }
  .read-output td.line-num {
    color: var(--t3);
    text-align: right;
    user-select: none;
    width: 32px;
    padding-right: var(--sp-2);
    font-size: 10px;
  }

  /* ── Result output ── */
  .result-output {
    padding: var(--sp-3) var(--sp-4);
    overflow-x: auto;
  }
  .result-pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 10.5px;
    line-height: 1.55;
    font-family: var(--mono);
  }

  .result-divider {
    height: 1px;
    background: var(--bd);
  }

  /* ── Streaming ── */
  .streaming-output {
    border-top: 1px solid var(--bd);
    padding: var(--sp-3) var(--sp-4);
  }
  .streaming-line {
    font-family: var(--mono);
    font-size: 10.5px;
    line-height: 1.55;
    color: var(--t2);
    margin: 0;
  }

  /* ── Modal ── */
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.65);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 999;
  }
  .modal {
    background: var(--bg1);
    border: 1px solid var(--bd);
    border-radius: var(--radius-lg);
    width: 90vw;
    max-width: 900px;
    height: 85vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-4) var(--sp-5);
    border-bottom: 1px solid var(--bd);
    flex-shrink: 0;
  }
  .modal-title {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    font-family: var(--mono);
    font-size: 13px;
    font-weight: 600;
    color: var(--t0);
  }
  .modal-close {
    background: none;
    border: none;
    color: var(--t2);
    font-size: 16px;
    cursor: pointer;
    padding: 0 var(--sp-2);
    line-height: 1;
  }
  .modal-close:hover {
    color: var(--t0);
  }
  .modal-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--sp-4) var(--sp-5);
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  /* ── Modal Card Sections ── */
  .modal-card-section {
    border: 1px solid var(--bd);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--bg1);
    flex-shrink: 0;
  }
  .modal-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-2) var(--sp-4);
    background: var(--bg2);
    border-bottom: 1px solid var(--bd);
  }
  .modal-card-label {
    font-family: var(--mono);
    font-size: 10px;
    font-weight: 600;
    color: var(--t2);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .modal-card-body {
    font-family: var(--mono);
    font-size: var(--sm);
    line-height: 1.6;
    overflow-x: auto;
  }
  .modal-card-body .diff-block .diff-line {
    padding: 0 var(--sp-4);
  }
  .modal-card-body .code-text {
    margin: 0;
    padding: var(--sp-3) var(--sp-4);
    font-family: var(--mono);
    font-size: var(--sm);
    line-height: 1.6;
    color: var(--ac);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .modal-card-body .code-text code {
    font-family: var(--mono);
  }
  .modal-card-body .read-output {
    padding: var(--sp-3) var(--sp-4);
  }
  .modal-card-body .result-output {
    padding: var(--sp-3) var(--sp-4);
  }
  .modal-card-body .result-pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: var(--sm);
    line-height: 1.6;
    font-family: var(--mono);
  }

  .card-copy-btn {
    display: flex;
    align-items: center;
    gap: 3px;
    background: transparent;
    border: 1px solid var(--bd1);
    color: var(--t2);
    font-size: 9.5px;
    cursor: pointer;
    padding: 1px 6px;
    border-radius: var(--radius-sm);
    transition: all 0.12s;
    font-family: var(--mono);
  }
  .card-copy-btn:hover {
    background: var(--bg3);
    color: var(--t0);
    border-color: var(--t2);
  }
</style>
