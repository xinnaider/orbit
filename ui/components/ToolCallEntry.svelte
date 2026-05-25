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
  import { detectLang, highlightCode } from '../lib/highlight';
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
  export let compact = false;

  $: toolState = resultEntry
    ? resultEntry.exitCode === 0 || resultEntry.exitCode == null
      ? 'done'
      : 'failed'
    : streamingEntries.length > 0
      ? 'working'
      : 'queued';

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

  function handleCopyResult() {
    if (!resultEntry?.output) return;
    if (isReadTool) {
      copyToClipboard(stripLineNumbers(resultEntry.output).code);
    } else {
      copyToClipboard(resultEntry.output);
    }
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

  // Real-time diff from streaming entries (visible while tool is running)
  $: streamEdits = streamingEntries
    .filter((s) => {
      const tool = ((s.tool ?? '') as string).toLowerCase();
      return tool === 'edit' && s.toolInput?.old_string && s.toolInput?.new_string;
    })
    .map((s) => {
      const rawChunks = diffLines(
        s.toolInput!.old_string as string,
        s.toolInput!.new_string as string
      );
      return {
        rawChunks,
        inline: buildInlineLines(rawChunks).slice(0, 6),
        name: (s.toolInput as Record<string, any>)?.file_path ?? 'file',
      };
    });

  $: streamWrites = streamingEntries
    .filter((s) => {
      const tool = ((s.tool ?? '') as string).toLowerCase();
      return tool === 'write' && s.toolInput?.content;
    })
    .map((s) => ({
      lines: (s.toolInput!.content as string).split('\n').map((text, i) => ({
        type: 'add' as const,
        text,
        lineNo: i + 1,
      })),
      name: (s.toolInput as Record<string, any>)?.file_path ?? 'file',
    }));

  $: hasStreamDiffs = streamEdits.length > 0 || streamWrites.length > 0;
  $: showStreamingBody = streamingEntries.length > 0 && !resultEntry;
  $: showBody = hasDetail || !!resultEntry?.output || showStreamingBody;
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

  function shortPath(p: string, tool?: string): string {
    if (tool === 'bash') {
      let out = p.replace(/['"]+/g, '').replace(/\s+/g, ' ');
      if (out.length > 50) out = out.slice(0, 47) + '...';
      return out;
    }
    let clean = p.replace(/['"]+/g, '').replace(/\\/g, '/');
    const parts = clean.split('/');
    let out = parts.length > 2 ? parts.slice(-2).join('/') : clean;
    if (out.length > 50) out = out.slice(0, 47) + '...';
    return out;
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

<div class="tc-card quiet-tool-card" class:compact>
  <div class="tc-header quiet-tool-head">
    <span
      class="tc-title"
      onclick={() => (modalOpen = true)}
      role="button"
      tabindex="0"
      onkeydown={(e) => e.key === 'Enter' && (modalOpen = true)}
    >
      <span class="tc-tool">{entry.tool ?? 'tool'}</span>
      {#if target}
        <span class="tc-sep">→</span>
        <span class="tc-target">{shortPath(target, toolClass)}</span>
      {/if}
    </span>
    <span class="tc-state" class:failed={toolState === 'failed'}>{toolState}</span>
    <span class="tc-actions">
      <button
        class="tc-expand tc-action--label"
        onclick={handleCopy}
        title="Copy command"
        aria-label="Copy command"><Copy size={10} /><span class="actxt">cmd</span></button
      >
      {#if resultEntry?.output}
        <button
          class="tc-expand tc-action--label"
          onclick={handleCopyResult}
          title="Copy output"
          aria-label="Copy output"><Copy size={10} /><span class="actxt">out</span></button
        >
      {/if}
      <button
        class="tc-expand"
        onclick={() => (modalOpen = true)}
        title="View full"
        aria-label="View full"><Maximize2 size={11} /></button
      >
    </span>
  </div>

  {#if showBody}
    <div class="tc-body quiet-tool-body">
      {#if hasEditDiff}
        <div class="diff-block">
          {#each inlineVisible as dl}
            <div class="diff-line {dl.type}">
              <span class="dl-num">{dl.lineNo}</span>
              <span class="dl-prefix"
                >{dl.type === 'add' ? '+' : dl.type === 'rem' ? '-' : ' '}</span
              >
              <span class="dl-code">{@html highlightCode(dl.text, lang)}</span>
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
              <span class="dl-code">{@html highlightCode(dl.text, lang)}</span>
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
          <pre class="bash-code"><code>{@html highlightCode(codeText, 'bash')}</code></pre>
        </div>
      {/if}

      {#if showStreamingBody}
        <div class="streaming-output">
          {#if hasStreamDiffs}
            {#each streamEdits as edit}
              <div class="stream-diff-preview">
                <div class="stream-diff-header">editing {edit.name}...</div>
                <div class="diff-block">
                  {#each edit.inline as dl}
                    <div class="diff-line {dl.type}">
                      <span class="dl-num">{dl.lineNo}</span>
                      <span class="dl-prefix">{dl.type === 'add' ? '+' : '-'}</span>
                      <span class="dl-code">{@html highlightCode(dl.text, lang)}</span>
                    </div>
                  {/each}
                </div>
              </div>
            {/each}
            {#each streamWrites as write}
              <div class="stream-diff-preview">
                <div class="stream-diff-header">creating {write.name}...</div>
                <div class="diff-block">
                  {#each write.lines.slice(0, 6) as dl}
                    <div class="diff-line add">
                      <span class="dl-num">{dl.lineNo}</span>
                      <span class="dl-prefix">+</span>
                      <span class="dl-code"
                        >{@html highlightCode(dl.text, detectLang(write.name))}</span
                      >
                    </div>
                  {/each}
                  {#if write.lines.length > 6}
                    <div class="diff-overflow">▸ +{write.lines.length - 6} linhas</div>
                  {/if}
                </div>
              </div>
            {/each}
          {:else}
            {#each streamingEntries as s}
              <pre class="streaming-line">{s.text}</pre>
            {/each}
          {/if}
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
                    <td class="line-code">{@html highlightCode(line, lang)}</td>
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
          {#if target}
            <span class="tc-sep">→</span>
            <span class="target mono">{shortPath(target, toolClass)}</span>
          {/if}
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
                    <span class="dl-code">{@html highlightCode(dl.text, lang)}</span>
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
                    <span class="dl-code">{@html highlightCode(dl.text, lang)}</span>
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
              <pre class="code-text"><code>{@html highlightCode(codeText, 'bash')}</code></pre>
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
                          <td class="line-code">{@html highlightCode(line, lang)}</td>
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
    overflow: hidden;
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
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    margin-right: auto;
  }
  .tc-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
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
    white-space: nowrap;
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
  .stream-diff-preview {
    margin-bottom: var(--sp-3);
  }
  .stream-diff-header {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--t3);
    margin-bottom: var(--sp-1);
    padding: var(--sp-1) var(--sp-2);
    background: var(--bg2);
    border-radius: var(--radius-sm);
  }
  .stream-diff-preview .diff-block {
    border: 0;
    margin: 0;
  }
  .stream-diff-preview .diff-overflow {
    font-size: 10px;
    color: var(--ac);
    padding: var(--sp-1) var(--sp-2);
    cursor: pointer;
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
    border-radius: var(--radius-sm);
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

  /* ── Quiet tool card ── */
  .quiet-tool-card {
    border: 1px solid color-mix(in srgb, var(--tool-fg), transparent 84%);
    background: var(--tool-bg);
    border-radius: var(--radius-md);
    overflow: hidden;
    max-width: 100%;
  }
  .quiet-tool-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--tool-fg), transparent 90%);
    color: var(--tool-fg);
    font-family: var(--mono);
    font-size: 11px;
  }
  .tc-state {
    color: var(--t2);
  }
  .tc-state.failed {
    color: var(--s-error);
  }
  .tc-expand {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
    color: var(--t3);
    cursor: pointer;
    font-size: 12px;
    line-height: 1;
    padding: 0;
    flex-shrink: 0;
    transition: all 0.12s;
  }
  .tc-expand:hover {
    border-color: var(--bd1);
    color: var(--t1);
    background: color-mix(in srgb, var(--t0), transparent 95%);
  }
  .tc-action--label {
    width: auto;
    gap: 4px;
    padding: 0 6px;
    font-size: 9px;
    font-family: var(--mono);
    font-weight: 500;
    background: color-mix(in srgb, var(--t0), transparent 94%);
    border-color: color-mix(in srgb, var(--t0), transparent 88%);
  }
  .tc-action--label:hover {
    background: color-mix(in srgb, var(--t0), transparent 88%);
  }
  .actxt {
    line-height: 1;
  }
  .quiet-tool-body {
    padding: 12px;
    background: rgba(0, 0, 0, 0.12);
    color: var(--t1);
    font-family: var(--mono);
    font-size: 11px;
    line-height: 1.55;
  }
  .quiet-tool-card.compact {
    border-radius: var(--radius-md);
  }
  .quiet-tool-card.compact .quiet-tool-head {
    padding: 8px 10px;
    font-size: 10px;
  }
  .quiet-tool-card.compact .quiet-tool-body {
    padding: 9px 10px;
    font-size: 10px;
  }
</style>
