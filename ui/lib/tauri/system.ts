import { getVersion as _getVersion } from '@tauri-apps/api/app';
import type { UpdateInfo } from '../types';
import { invoke, IS_MOCK } from './invoke';

export interface ClaudeCheck {
  found: boolean;
  path: string | null;
  searchedPath?: string;
  hint?: string;
}

export interface SpawnDiagnostic {
  claudeFound: boolean;
  claudePath: string | null;
  whereOutput: string;
  versionOutput: string;
  augmentedPath: string;
  processPath: string;
}

export interface ClaudeUsageStats {
  weeklyTokens: number;
  todayTokens: number;
  weeklyMessages: number;
  todayMessages: number;
}

export interface RateLimits {
  cost: number;
  fiveHourPct: number;
  fiveHourReset: number;
  sevenDayPct: number;
  sevenDayReset: number;
  contextPct: number;
}

export async function checkClaude(): Promise<ClaudeCheck> {
  return await invoke('check_claude');
}

export async function diagnoseSpawn(): Promise<SpawnDiagnostic> {
  return await invoke('diagnose_spawn');
}

export async function getAppVersion(): Promise<string> {
  if (IS_MOCK) return '0.0.0';
  return _getVersion();
}

export async function checkUpdate(): Promise<UpdateInfo | null> {
  return await invoke<UpdateInfo | null>('check_update');
}

export async function installUpdate(): Promise<void> {
  await invoke('install_update');
}

export async function getChangelog(): Promise<string> {
  return await invoke<string>('get_changelog');
}

export async function getClaudeUsageStats(): Promise<ClaudeUsageStats> {
  return await invoke('get_claude_usage_stats');
}

export async function getRateLimits(pid: number | null): Promise<RateLimits> {
  return await invoke('get_rate_limits', { pid });
}
