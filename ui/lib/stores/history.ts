const MAX_HISTORY = 50;
const STORAGE_KEY = 'orbit:message-history';

interface SessionHistory {
  messages: string[];
  cursor: number;
  savedText: string;
}

function loadAll(): Record<string, string[]> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function persistAll(all: Record<string, string[]>) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(all));
  } catch {
    // localStorage full
  }
}

function createHistory() {
  const all = loadAll();
  const sessions: Record<string, SessionHistory> = {};

  function get(sessionId: string): SessionHistory {
    if (!sessions[sessionId]) {
      sessions[sessionId] = {
        messages: all[sessionId] ?? [],
        cursor: -1,
        savedText: '',
      };
    }
    return sessions[sessionId];
  }

  function push(sessionId: string, message: string) {
    const h = get(sessionId);
    if (h.messages.length > 0 && h.messages[0] === message) return;
    h.messages.unshift(message);
    if (h.messages.length > MAX_HISTORY) h.messages.length = MAX_HISTORY;
    h.cursor = -1;
    h.savedText = '';
    all[sessionId] = h.messages;
    persistAll(all);
  }

  function up(sessionId: string, currentText: string): string | null {
    const h = get(sessionId);
    if (h.cursor === -1) {
      h.savedText = currentText;
    }
    if (h.cursor < h.messages.length - 1) {
      h.cursor++;
      return h.messages[h.cursor];
    }
    return null;
  }

  function down(sessionId: string): string | null {
    const h = get(sessionId);
    if (h.cursor > 0) {
      h.cursor--;
      return h.messages[h.cursor];
    }
    if (h.cursor === 0) {
      h.cursor = -1;
      return h.savedText;
    }
    return null;
  }

  function resetCursor(sessionId: string) {
    const h = get(sessionId);
    h.cursor = -1;
    h.savedText = '';
  }

  return { push, up, down, resetCursor };
}

export const messageHistory = createHistory();
