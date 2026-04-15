<script lang="ts">
  export let sshHost: string;
  export let sshUser: string;
  export let sshPassword: string;
  export let loading: boolean;
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
  <label class="label" for="ns-ssh-pw"
    >password <span class="key-hint">(optional — uses SSH key if empty)</span></label
  >
  <input
    id="ns-ssh-pw"
    class="input"
    type="password"
    bind:value={sshPassword}
    placeholder="leave empty for key auth"
    disabled={loading}
  />
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
</style>
