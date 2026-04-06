<script lang="ts">
  import { onMount } from 'svelte';
  import {
    sessions, selectedSessionId, upsertSession, updateSessionState
  } from './lib/stores/sessions';
  import { journal } from './lib/stores/journal';
  import {
    listSessions,
    onSessionCreated,
    onSessionOutput,
    onSessionState,
    onSessionStopped,
  } from './lib/tauri';
  import Sidebar from './components/Sidebar.svelte';
  import CentralPanel from './components/CentralPanel.svelte';
  import RightPanel from './components/RightPanel.svelte';

  let prevStatuses: Record<number, string> = {};
  let audioCtx: AudioContext | null = null;

  function playNotificationBeep() {
    try {
      if (!audioCtx || audioCtx.state === 'closed') {
        audioCtx = new AudioContext();
      }
      const osc = audioCtx.createOscillator();
      const gain = audioCtx.createGain();
      osc.connect(gain);
      gain.connect(audioCtx.destination);
      osc.frequency.value = 800;
      osc.type = 'sine';
      gain.gain.value = 0.3;
      gain.gain.exponentialRampToValueAtTime(0.001, audioCtx.currentTime + 0.2);
      osc.start(audioCtx.currentTime);
      osc.stop(audioCtx.currentTime + 0.2);
    } catch {
      // Audio not available
    }
  }

  onMount(async () => {
    // Load existing sessions on startup
    const existing = await listSessions();
    sessions.set(existing);
    if (existing.length > 0 && !$selectedSessionId) {
      selectedSessionId.set(existing[0].id);
    }

    // session:created — new session spawned
    const unCreated = onSessionCreated((session) => {
      sessions.update(list => upsertSession(list, session));
      if (!$selectedSessionId) selectedSessionId.set(session.id);
    });

    // session:output — new journal entry
    const unOutput = onSessionOutput(({ sessionId, entry }) => {
      journal.update(map => {
        const entries = map.get(sessionId) ?? [];
        return new Map(map).set(sessionId, [...entries, entry]);
      });
    });

    // session:state — status/token update
    const unState = onSessionState((payload) => {
      const prev = prevStatuses[payload.sessionId];
      if (payload.status === 'input' && prev && prev !== 'input') {
        playNotificationBeep();
      }
      prevStatuses[payload.sessionId] = payload.status;

      sessions.update(list => updateSessionState(list, payload.sessionId, {
        status: payload.status as any,
        tokens: payload.tokens,
        contextPercent: payload.contextPercent,
        pendingApproval: payload.pendingApproval,
        miniLog: payload.miniLog,
      }));
    });

    // session:stopped
    const unStopped = onSessionStopped((sessionId) => {
      sessions.update(list => updateSessionState(list, sessionId, { status: 'completed' }));
    });

    return () => {
      Promise.all([unCreated, unOutput, unState, unStopped]).then(fns => fns.forEach(fn => fn()));
    };
  });
</script>

<div class="app-layout">
  <Sidebar />
  <CentralPanel />
  <RightPanel />
</div>

<style>
  .app-layout {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }
</style>
