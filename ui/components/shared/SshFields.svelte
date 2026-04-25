<script lang="ts">
  import { HAS_TAURI } from '../../lib/tauri/invoke';

  export let sshHost: string;
  export let sshUser: string;
  export let sshKeyPath: string;
  export let loading: boolean;

  async function browseKey() {
    if (!HAS_TAURI) return;
    const { open } = await import('@tauri-apps/plugin-dialog');
    const sel = await open({
      multiple: false,
      filters: [
        { name: 'SSH Keys', extensions: ['pem', 'key', 'rsa', 'pub', 'id_rsa', 'id_ed25519'] },
      ],
    });
    if (sel && typeof sel === 'string') sshKeyPath = sel;
  }
</script>

<div class="field">
  <label class="label" for="ns-ssh-host">host</label>
  <input
    id="ns-ssh-host"
    class="input"
    type="text"
    bind:value={sshHost}
    placeholder="vps.example.com"
    disabled={loading}
  />
</div>

<div class="field">
  <label class="label" for="ns-ssh-user">user</label>
  <input
    id="ns-ssh-user"
    class="input"
    type="text"
    bind:value={sshUser}
    placeholder="ubuntu"
    disabled={loading}
  />
</div>

<div class="field">
  <label class="label" for="ns-ssh-key"
    >SSH key <span class="key-hint">(private key file)</span></label
  >
  <div class="key-row">
    <input
      id="ns-ssh-key"
      class="input"
      type="text"
      bind:value={sshKeyPath}
      placeholder="~/.ssh/id_rsa"
      disabled={loading}
    />
    <button class="browse" on:click={browseKey} disabled={loading} title="browse">⌘</button>
  </div>
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }
  .label {
    font-size: var(--xs);
    color: var(--t2);
    letter-spacing: 0.06em;
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }
  .input {
    background: var(--bg2);
    border: 1px solid var(--bd1);
    border-radius: var(--radius-sm);
    color: var(--t0);
    font-size: var(--md);
    padding: var(--sp-3) var(--sp-4);
    outline: none;
    width: 100%;
    transition: border-color 0.15s;
  }
  .input:focus {
    border-color: var(--bd2);
  }
  .input:disabled {
    opacity: 0.5;
  }
  .key-hint {
    font-weight: normal;
    color: var(--t3);
    font-size: 10px;
  }
  .key-row {
    display: flex;
    gap: var(--sp-3);
  }
  .key-row .input {
    flex: 1;
  }
  .browse {
    background: var(--bg2);
    border: 1px solid var(--bd1);
    color: var(--t1);
    border-radius: var(--radius-sm);
    padding: 0 var(--sp-5);
    font-size: var(--base);
    flex-shrink: 0;
  }
  .browse:hover {
    border-color: var(--bd2);
    color: var(--t0);
  }
</style>
