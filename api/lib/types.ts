export type AgentStatus = 'working' | 'input' | 'idle' | 'new';

export interface TokenUsage {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
}

export interface MiniLogEntry {
  tool: string;
  target: string;
  result: string | null;
  success: boolean | null;
}

export interface SubagentInfo {
  id: string;
  agentType: string;
  description: string;
  status: string;
}

export interface AgentState {
  sessionId: string;
  project: string;
  cwd: string;
  gitBranch: string | null;
  status: AgentStatus;
  model: string | null;
  modelDisplay: string;
  tokens: TokenUsage;
  contextPercent: number;
  subagents: SubagentInfo[];
  miniLog: MiniLogEntry[];
  pendingApproval: string | null;
  pid: number | null;
  startedAt: number;
}

export type JournalEntryType =
  | 'user'
  | 'thinking'
  | 'assistant'
  | 'toolCall'
  | 'toolResult'
  | 'system'
  | 'progress';

export interface JournalEntry {
  sessionId: string;
  timestamp: string;
  entryType: JournalEntryType;
  text: string | null;
  thinking: string | null;
  thinkingDuration: number | null;
  tool: string | null;
  toolInput: Record<string, unknown> | null;
  output: string | null;
  exitCode: number | null;
  linesChanged: { added: number; removed: number } | null;
}

export interface SlashCommand {
  cmd: string;
  desc: string;
  category: string;
}

export interface TaskItem {
  id: string;
  subject: string;
  description: string;
  activeForm: string | null;
  status: 'pending' | 'in_progress' | 'completed';
  blocks: string[];
  blockedBy: string[];
}

export type DetailLevel = 'compact' | 'full' | 'raw';
export type RightPanelTab = 'agents' | 'tasks' | 'stats';

export interface UpdateInfo {
  version: string;
  body: string;
  currentVersion: string;
}
