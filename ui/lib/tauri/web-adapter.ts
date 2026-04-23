/**
 * Web adapter — replaces Tauri invoke/listen when running in a browser.
 * Maps Tauri command names to HTTP REST calls against the Orbit HTTP API.
 */

const TOKEN_KEY = 'orbit_api_token';

export function getStoredToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function setStoredToken(token: string): void {
  localStorage.setItem(TOKEN_KEY, token);
}

export function clearStoredToken(): void {
  localStorage.removeItem(TOKEN_KEY);
}

function headers(): Record<string, string> {
  const token = getStoredToken();
  return {
    'Content-Type': 'application/json',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };
}

async function apiGet<T>(path: string): Promise<T> {
  const res = await fetch(`/api${path}`, { headers: headers() });
  if (res.status === 401) throw new Error('UNAUTHORIZED');
  if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
  return res.json();
}

async function apiPost<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`/api${path}`, {
    method: 'POST',
    headers: headers(),
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (res.status === 401) throw new Error('UNAUTHORIZED');
  if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
  return res.json();
}

async function apiDelete<T>(path: string): Promise<T> {
  const res = await fetch(`/api${path}`, {
    method: 'DELETE',
    headers: headers(),
  });
  if (res.status === 401) throw new Error('UNAUTHORIZED');
  if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
  return res.json();
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Args = Record<string, any>;

/**
 * Route a Tauri command to the matching HTTP endpoint.
 * Commands that don't have a REST mapping return a sensible default.
 */
export async function webInvoke<T>(cmd: string, args?: Args): Promise<T> {
  switch (cmd) {
    // ── Sessions ──────────────────────────────────────────────
    case 'list_sessions':
      return apiGet('/sessions') as Promise<T>;

    case 'create_session': {
      const a = args!;
      return apiPost('/sessions', {
        cwd: a.cwd,
        prompt: a.prompt,
        provider: a.provider,
        model: a.model,
        name: a.name,
        parentSessionId: a.parentSessionId,
      }) as Promise<T>;
    }

    case 'stop_session':
      return apiPost(`/sessions/${args!.sessionId}/stop`) as Promise<T>;

    case 'send_session_message':
      return apiPost(`/sessions/${args!.sessionId}/message`, {
        message: args!.message,
      }) as Promise<T>;

    case 'rename_session':
      return apiPost(`/sessions/${args!.sessionId}/rename`, {
        name: args!.name,
      }) as Promise<T>;

    case 'delete_session':
      return apiDelete(`/sessions/${args!.sessionId}`) as Promise<T>;

    case 'get_session_journal':
      return apiGet(`/sessions/${args!.sessionId}/journal`) as Promise<T>;

    case 'get_session_journal_page':
      return apiGet(
        `/sessions/${args!.sessionId}/journal?offset=${args!.offset ?? 0}&limit=${args!.limit ?? 200}`
      ) as Promise<T>;

    // ── Providers ─────────────────────────────────────────────
    case 'get_providers':
      return apiGet('/providers') as Promise<T>;

    // ── Subagents ─────────────────────────────────────────────
    case 'get_subagents':
      return apiGet(`/sessions/${args!.sessionId}/subagents`) as Promise<T>;

    // ── Health / system ───────────────────────────────────────
    case 'check_claude':
      return apiGet('/health').then(() => ({ available: true })) as Promise<T>;

    case 'get_changelog':
      return '' as unknown as T;

    // ── HTTP API management (these still work via REST in web mode) ──
    case 'generate_api_key':
      return apiPost('/api-keys', { label: args!.label }) as Promise<T>;

    case 'list_api_keys':
      return apiGet('/api-keys') as Promise<T>;

    case 'revoke_api_key':
      return apiDelete(`/api-keys/${args!.id}`) as Promise<T>;

    case 'get_http_settings':
      return apiGet('/settings/http') as Promise<T>;

    case 'set_http_settings':
      return apiPost('/settings/http', {
        enabled: args!.enabled,
        host: args!.host,
        port: args!.port,
      }) as Promise<T>;

    // ── Commands that are no-ops in web mode ──────────────────
    case 'diagnose_spawn':
    case 'check_update':
    case 'install_update':
    case 'get_claude_usage_stats':
    case 'get_rate_limits':
    case 'setup_orchestration':
    case 'check_orchestration':
    case 'check_env_var':
    case 'diagnose_provider':
    case 'test_ssh':
    case 'save_provider_key':
    case 'load_provider_key':
    case 'delete_provider_key':
    case 'clear_attention':
    case 'respond_permission':
    case 'update_session_model':
    case 'update_session_effort':
    case 'set_session_api_key':
    case 'create_project':
    case 'list_projects':
    case 'list_project_files':
    case 'get_subagent_journal':
    case 'get_slash_commands':
    case 'get_tasks':
    case 'read_file_content':
    case 'pty_create':
    case 'pty_write':
    case 'pty_resize':
    case 'pty_kill':
      return null as unknown as T;

    default:
      console.warn(`[orbit:web] unhandled command: ${cmd}`);
      return null as unknown as T;
  }
}

// ── WebSocket event adapter ─────────────────────────────────────

type EventCallback = (payload: unknown) => void;
const eventListeners = new Map<string, Set<EventCallback>>();
let ws: WebSocket | null = null;
let wsReconnectTimer: ReturnType<typeof setTimeout> | null = null;

function connectWebSocket() {
  const token = getStoredToken();
  if (!token) return;

  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const url = `${protocol}//${location.host}/api/ws?token=${encodeURIComponent(token)}`;

  ws = new WebSocket(url);

  ws.onmessage = (e) => {
    try {
      const event = JSON.parse(e.data);
      const sessionId = event.session_id ?? event.sessionId;
      const eventType: string = event.event_type ?? event.eventType ?? '';
      const data = event.data ?? {};

      // Dispatch to matching listeners
      const listeners = eventListeners.get(eventType);
      if (listeners) {
        const payload = { ...data, sessionId };
        listeners.forEach((cb) => cb(payload));
      }
    } catch {
      // ignore parse errors
    }
  };

  ws.onclose = () => {
    ws = null;
    wsReconnectTimer = setTimeout(connectWebSocket, 3000);
  };

  ws.onerror = () => {
    ws?.close();
  };
}

export function webListen(event: string, cb: (e: { payload: unknown }) => void): () => void {
  // Ensure WS is connected
  if (!ws && !wsReconnectTimer) {
    connectWebSocket();
  }

  if (!eventListeners.has(event)) {
    eventListeners.set(event, new Set());
  }
  const wrappedCb: EventCallback = (payload) => cb({ payload });
  eventListeners.get(event)!.add(wrappedCb);

  return () => {
    eventListeners.get(event)?.delete(wrappedCb);
  };
}

export function disconnectWebSocket() {
  if (wsReconnectTimer) clearTimeout(wsReconnectTimer);
  ws?.close();
  ws = null;
}
