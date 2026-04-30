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

// ── Mesh types ─────────────────────────────────────────────

export type Floor = {
  id: number;
  name: string;
  position: number;
  createdAt: string;
};

export type AgentTemplate = {
  id: number;
  floorId: number;
  name: string;
  prePrompt: string;
  model: string | null;
  provider: string;
  useWorktree: boolean;
  createdAt: string;
  updatedAt: string;
};

export type Graph = {
  id: number;
  floorId: number;
  name: string;
  entryNodeId: number | null;
  provider: string;
  createdAt: string;
  updatedAt: string;
};

export type GraphNode = {
  id: number;
  graphId: number;
  templateId: number;
  displayName: string;
  x: number;
  y: number;
  width?: number | null;
  height?: number | null;
};

export type GraphEdge = {
  id: number;
  graphId: number;
  fromNodeId: number;
  toNodeId: number;
  fromHandle?: string | null;
  toHandle?: string | null;
};

export type MeshNote = {
  nodeId: number;
  graphId: number;
  name: string;
  content: string;
  x: number;
  y: number;
  width?: number | null;
  height?: number | null;
  updatedAt: string;
};

export type CanvasAnnotation = {
  id: number;
  graphId: number;
  kind: 'path' | 'sticky' | 'arrow';
  payload: string;
  zIndex: number;
};

export type NewAnnotation = {
  kind: 'path' | 'sticky' | 'arrow';
  payload: string;
  zIndex: number;
};

export type Skill = {
  slug: string;
  name: string;
  description: string;
  path: string;
  content: string;
};

export type Run = {
  id: number;
  graphId: number;
  entryNodeId: number;
  initialPrompt: string | null;
  status: 'pending' | 'running' | 'completed' | 'stopped' | 'error';
  maxDepth: number;
  timeoutSecs: number;
  maxLoopCount: number;
  ombroEnabled: boolean;
  startedAt: string | null;
  finishedAt: string | null;
  createdAt: string;
};

export type RunSession = {
  id: number;
  runId: number;
  nodeId: number;
  sessionId: number;
};
