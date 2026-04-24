<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { setStoredToken } from '../lib/tauri/web-adapter';

  const dispatch = createEventDispatcher<{ authenticated: void }>();

  let token = '';
  let error = '';
  let loading = false;

  // Check URL params for token (QR code flow)
  const urlToken = new URLSearchParams(window.location.search).get('token');
  if (urlToken) {
    setStoredToken(urlToken);
    // Clean URL
    window.history.replaceState({}, '', window.location.pathname);
    dispatch('authenticated');
  }

  async function submit() {
    if (!token.trim()) return;
    loading = true;
    error = '';

    try {
      const res = await fetch('/api/health', {
        headers: { Authorization: `Bearer ${token.trim()}` },
      });

      if (res.ok) {
        setStoredToken(token.trim());
        dispatch('authenticated');
      } else {
        error = 'invalid API key';
      }
    } catch {
      error = 'cannot connect to Orbit';
    } finally {
      loading = false;
    }
  }
</script>

<div class="login-screen">
  <div class="login-card">
    <div class="beta-banner">
      <span class="beta-pill">mobile beta</span>
      <p>Phone access is in testing. Some screens and actions may not work as expected yet.</p>
    </div>

    <div class="logo">
      <svg width="32" height="32" viewBox="0 0 100 100" fill="none">
        <circle cx="50" cy="50" r="45" stroke="var(--ac)" stroke-width="3" fill="none"></circle>
        <circle cx="50" cy="50" r="8" fill="var(--ac)"></circle>
        <circle cx="50" cy="18" r="5" fill="var(--ac)" opacity="0.6"></circle>
      </svg>
      <span class="logo-text">orbit</span>
    </div>

    <p class="subtitle">paste your access key to connect</p>

    <form on:submit|preventDefault={submit}>
      <input
        class="input"
        type="password"
        bind:value={token}
        placeholder="orbit_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
        disabled={loading}
        autocomplete="off"
      />

      {#if error}
        <span class="error">{error}</span>
      {/if}

      <button class="btn" type="submit" disabled={loading || !token.trim()}>
        {loading ? 'connecting...' : 'connect'}
      </button>
    </form>

    <p class="hint">generate a key in Orbit desktop: sidebar footer &rarr; Phone &rarr; Advanced</p>
  </div>
</div>

<style>
  .login-screen {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    background: var(--bg0, #0a0a0a);
    padding: 20px;
  }
  .login-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
    max-width: 380px;
    width: 100%;
  }
  .beta-banner {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    width: 100%;
    box-sizing: border-box;
    padding: 12px 14px;
    border-radius: 10px;
    border: 1px solid rgba(245, 166, 35, 0.28);
    background: rgba(245, 166, 35, 0.08);
  }
  .beta-banner p {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--t1, #d0d0d0);
  }
  .beta-pill {
    flex-shrink: 0;
    padding: 4px 7px;
    border-radius: 999px;
    background: rgba(245, 166, 35, 0.14);
    color: var(--warning, #f5a623);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .logo {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .logo-text {
    font-size: 24px;
    font-weight: 600;
    color: var(--t0, #fff);
    font-family: var(--mono, monospace);
    letter-spacing: 0.04em;
  }
  .subtitle {
    font-size: 13px;
    color: var(--t2, #888);
    margin: 0;
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
  }
  .input {
    background: var(--bg2, #1a1a1a);
    border: 1px solid var(--bd1, #333);
    border-radius: 6px;
    color: var(--t0, #fff);
    font-size: 13px;
    padding: 10px 14px;
    outline: none;
    width: 100%;
    font-family: var(--mono, monospace);
    transition: border-color 0.15s;
    box-sizing: border-box;
  }
  .input:focus {
    border-color: var(--ac, #00d47e);
  }
  .error {
    font-size: 12px;
    color: var(--error, #ef4444);
  }
  .btn {
    background: var(--ac-d, rgba(0, 212, 126, 0.1));
    border: 1px solid var(--ac, #00d47e);
    border-radius: 6px;
    color: var(--ac, #00d47e);
    font-size: 13px;
    padding: 10px 20px;
    cursor: pointer;
    font-family: var(--mono, monospace);
    transition: all 0.15s;
  }
  .btn:hover {
    background: rgba(0, 212, 126, 0.18);
  }
  .btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .hint {
    font-size: 11px;
    color: var(--t3, #555);
    text-align: center;
    margin: 0;
    line-height: 1.5;
  }
</style>
