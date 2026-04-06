<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    sessions, selectedSessionId,
    upsertSession, updateSessionState, getSelectedSession
  } from './lib/stores/sessions';
  import { journal } from './lib/stores/journal';
  import {
    listSessions, checkClaude,
    onSessionCreated, onSessionOutput, onSessionState,
    onSessionStopped, onSessionRunning, onSessionError,
  } from './lib/tauri';
  import type { ClaudeCheck } from './lib/tauri';
  import Sidebar from './components/Sidebar.svelte';
  import CentralPanel from './components/CentralPanel.svelte';
  import MetaPanel from './components/MetaPanel.svelte';

  let prevStatuses: Record<number, string> = {};
  let audioCtx: AudioContext | null = null;
  let claudeCheck: ClaudeCheck | null = null;
  let unlisteners: Array<() => void> = [];

  function beep() {
    try {
      if (!audioCtx || audioCtx.state === 'closed') audioCtx = new AudioContext();
      const osc = audioCtx.createOscillator();
      const gain = audioCtx.createGain();
      osc.connect(gain); gain.connect(audioCtx.destination);
      osc.frequency.value = 880; osc.type = 'sine';
      gain.gain.value = 0.15;
      gain.gain.exponentialRampToValueAtTime(0.001, audioCtx.currentTime + 0.15);
      osc.start(); osc.stop(audioCtx.currentTime + 0.15);
    } catch {}
  }

  onMount(async () => {
    const [existing, check] = await Promise.all([listSessions(), checkClaude()]);
    claudeCheck = check;
    sessions.set(existing);
    if (existing.length > 0 && !$selectedSessionId) selectedSessionId.set(existing[0].id);

    const u1 = onSessionCreated(s => {
      sessions.update(l => upsertSession(l, s));
      if (!$selectedSessionId) selectedSessionId.set(s.id);
    });

    const u2 = onSessionOutput(({ sessionId, entry }) => {
      journal.update(map => new Map(map).set(sessionId, [...(map.get(sessionId) ?? []), entry]));
    });

    const u3 = onSessionState(p => {
      const prev = prevStatuses[p.sessionId];
      if (p.status === 'input' && prev && prev !== 'input') beep();
      prevStatuses[p.sessionId] = p.status;
      sessions.update(l => updateSessionState(l, p.sessionId, {
        status: p.status as any,
        tokens: p.tokens,
        contextPercent: p.contextPercent,
        pendingApproval: p.pendingApproval,
        miniLog: p.miniLog,
      }));
    });

    const u4 = onSessionStopped(id => {
      sessions.update(l => updateSessionState(l, id, { status: 'completed' }));
    });

    const u5 = onSessionRunning((id, pid) => {
      sessions.update(l => updateSessionState(l, id, { status: 'running', pid }));
    });

    const u6 = onSessionError((id, error) => {
      sessions.update(l => updateSessionState(l, id, { status: 'error' }));
      console.error('session:error', id, error);
    });

    // Resolve all unlisten functions and store for cleanup
    Promise.all([u1, u2, u3, u4, u5, u6]).then(fns => { unlisteners = fns; });
  });

  onDestroy(() => unlisteners.forEach(fn => fn()));

  $: selected = getSelectedSession($sessions, $selectedSessionId);
</script>

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
            <div class="warn-hint">{claudeCheck.hint ?? 'npm install -g @anthropic-ai/claude-code'}</div>
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

<style>
  .layout {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
  }
  .empty-hint {
    font-size: var(--sm);
    color: var(--t2);
    letter-spacing: 0.02em;
  }
  .claude-warn {
    display: flex; align-items: flex-start; gap: 10px;
    background: rgba(224,72,72,0.07);
    border: 1px solid rgba(224,72,72,0.25);
    border-radius: 4px; padding: 14px 18px;
    max-width: 360px;
  }
  .warn-icon { color: var(--s-error); font-size: 16px; flex-shrink: 0; margin-top: 1px; }
  .warn-title { font-size: var(--md); color: var(--s-error); margin-bottom: 4px; }
  .warn-hint { font-size: var(--xs); color: var(--t1); font-style: italic; }
</style>
