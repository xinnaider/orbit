<script lang="ts">
  import type { JournalEntry } from '../lib/types';
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

  export let entry: JournalEntry;
  export let resultEntry: JournalEntry | null = null;

  $: toolClass = (entry.tool ?? '').toLowerCase();
  $: target = extractTarget(entry);
  $: timeStr = entry.timestamp.slice(11, 16);
  $: hasEditDiff = toolClass === 'edit' && entry.toolInput?.old_string && entry.toolInput?.new_string;
  $: hasWriteContent = toolClass === 'write' && entry.toolInput?.content;
  $: hasBashCommand = toolClass === 'bash' && entry.toolInput?.command;
  $: isReadTool = toolClass === 'read';
  $: hasDetail = hasEditDiff || hasWriteContent || hasBashCommand;

  $: lang = detectLang(target);

  // Diff lines
  $: oldLines = hasEditDiff ? (entry.toolInput!.old_string as string).split('\n') : [];
  $: newLines = hasEditDiff ? (entry.toolInput!.new_string as string).split('\n') : [];
  $: allDiffLines = [...oldLines.map(l => ({ type: 'rem' as const, text: l })), ...newLines.map(l => ({ type: 'add' as const, text: l }))];

  // Code text
  $: codeText = hasBashCommand ? (entry.toolInput!.command as string) : hasWriteContent ? (entry.toolInput!.content as string) : '';

  const toolIcons: Record<string, string> = {
    read: '📄', edit: '✏️', write: '📝', bash: '⚡',
    grep: '🔍', glob: '📁', agent: '🤖', skill: '🔧',
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
      js: 'javascript', jsx: 'javascript', mjs: 'javascript',
      ts: 'typescript', tsx: 'typescript',
      py: 'python', rs: 'rust', css: 'css',
      html: 'html', svelte: 'svelte', vue: 'html',
      json: 'json', yaml: 'yaml', yml: 'yaml',
      sh: 'bash', bash: 'bash', zsh: 'bash',
      md: 'markdown', toml: 'yaml',
    };
    return map[ext] ?? '';
  }

  function doHighlight(code: string, language: string): string {
    if (language && hljs.getLanguage(language)) {
      return hljs.highlight(code, { language }).value;
    }
    return hljs.highlightAuto(code).value;
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
  </div>

  {#if hasDetail || resultEntry?.output}
    <div class="detail">
      <div class="code-card">
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
          <pre class="code-inner code-text"><code>{@html doHighlight(codeText, hasBashCommand ? 'bash' : lang)}</code></pre>
        {/if}

        {#if resultEntry?.output}
          {#if hasDetail}
            <div class="result-divider"></div>
          {/if}
          {#if isReadTool}
            {@const parsed = stripLineNumbers(resultEntry.output)}
            <div class="code-inner read-output">
              <table class="read-table">
                {#each parsed.code.split('\n') as line, li}
                  <tr>
                    <td class="line-num">{parsed.lineNums[li] ?? ''}</td>
                    <td class="line-code">{@html doHighlight(line, lang)}</td>
                  </tr>
                {/each}
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
  .icon { font-size: 12px; flex-shrink: 0; }
  .tool { font-weight: 600; flex-shrink: 0; }
  .tool.read, .tool.grep, .tool.glob { color: var(--blue); }
  .tool.edit, .tool.write { color: var(--orange); }
  .tool.bash { color: var(--green); }
  .tool.agent { color: var(--purple); }
  .tool.skill { color: var(--pink); }
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
  .changes { display: flex; gap: 3px; flex-shrink: 0; }
  .added { color: var(--green); font-size: 11px; }
  .removed { color: var(--red); font-size: 11px; }

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
  .diff-line.rem .diff-prefix { color: var(--red); }
  .diff-line.add .diff-prefix { color: var(--green); }
  .diff-code { flex: 1; min-width: 0; }

  /* highlight.js tokens */
  .detail :global(.hljs-keyword),
  .detail :global(.hljs-selector-tag),
  .detail :global(.hljs-built_in) { color: var(--hl-keyword, #c678dd); }

  .detail :global(.hljs-string),
  .detail :global(.hljs-attr),
  .detail :global(.hljs-addition) { color: var(--hl-string, #98c379); }

  .detail :global(.hljs-number),
  .detail :global(.hljs-literal) { color: var(--hl-number, #d19a66); }

  .detail :global(.hljs-comment),
  .detail :global(.hljs-quote) { color: var(--hl-comment, #5c6370); font-style: italic; }

  .detail :global(.hljs-function),
  .detail :global(.hljs-title) { color: var(--hl-function, #61afef); }

  .detail :global(.hljs-type),
  .detail :global(.hljs-title.class_) { color: var(--hl-type, #e5c07b); }

  .detail :global(.hljs-variable),
  .detail :global(.hljs-template-variable) { color: var(--hl-variable, #e06c75); }

  .detail :global(.hljs-meta),
  .detail :global(.hljs-selector-class) { color: var(--hl-meta, #61afef); }

  .detail :global(.hljs-tag) { color: var(--hl-tag, #e06c75); }
  .detail :global(.hljs-name) { color: var(--hl-tag, #e06c75); }
  .detail :global(.hljs-attribute) { color: var(--hl-attr, #d19a66); }

  .detail :global(.hljs-params) { color: var(--text-primary); }
  .detail :global(.hljs-punctuation) { color: var(--text-secondary); }
</style>
