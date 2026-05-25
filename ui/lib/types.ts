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
  seq: number;
  epoch: string;
  /** Client-only: render system line as error styling in the feed */
  feedError?: boolean;
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

export type AttentionReason = 'permission' | 'completed' | 'error' | 'rateLimit';

export interface AttentionState {
  requiresAttention: boolean;
  reason: AttentionReason | null;
  since: string | null;
}

export interface RateLimitInfo {
  status: string;
  rateLimitType: string;
  utilization: number;
  resetsAt: number | null;
  isUsingOverage: boolean;
  surpassedThreshold: number;
}

export interface UpdateInfo {
  version: string;
  body: string;
  currentVersion: string;
}

export interface GitSnapshot {
  cwd: string;
  branch: string | null;
  upstream: string | null;
  isDirty: boolean;
  fileCount: number;
  files: string[];
  statusOutput: string | null;
  error: string | null;
}
