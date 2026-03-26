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

export type JournalEntryType = 'user' | 'thinking' | 'assistant' | 'toolCall' | 'toolResult' | 'system';

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

export type DetailLevel = 'compact' | 'full' | 'raw';
export type RightPanelTab = 'agents' | 'tasks' | 'stats';
