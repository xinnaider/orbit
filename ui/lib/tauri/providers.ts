import { invoke } from './invoke';

export interface ModelInfo {
  id: string;
  name: string;
  context: number | null;
  output: number | null;
}

export interface SubProvider {
  id: string;
  name: string;
  env: string[];
  configured: boolean;
  models: ModelInfo[];
}

export interface CliBackend {
  id: string;
  name: string;
  cliName: string;
  cliAvailable: boolean;
  installHint: string;
  supportsEffort: boolean;
  supportsSsh: boolean;
  supportsSubagents: boolean;
  hasSubProviders: boolean;
  models: ModelInfo[];
  subProviders: SubProvider[];
}

export interface SshDiagnostic {
  ok: boolean;
  latencyMs: number;
  error: string;
}

export interface ProviderDiagnostic {
  backend: string;
  cliName: string;
  found: boolean;
  path: string | null;
  version: string | null;
  installHint: string;
  ssh: SshDiagnostic | null;
  projectDirOk: boolean | null;
}

export interface SshTestResult {
  ok: boolean;
  latencyMs: number;
  error: string;
}

export async function getProviders(): Promise<CliBackend[]> {
  return await invoke('get_providers');
}

export async function checkEnvVar(name: string): Promise<boolean> {
  return await invoke('check_env_var', { name });
}

export async function diagnoseProvider(
  backend: string,
  opts?: {
    projectPath?: string;
    sshHost?: string;
    sshUser?: string;
    sshPassword?: string;
  }
): Promise<ProviderDiagnostic> {
  return await invoke('diagnose_provider', {
    backend,
    projectPath: opts?.projectPath ?? null,
    sshHost: opts?.sshHost ?? null,
    sshUser: opts?.sshUser ?? null,
    sshPassword: opts?.sshPassword ?? null,
  });
}

export async function testSsh(
  host: string,
  user: string,
  password?: string
): Promise<SshTestResult> {
  return await invoke('test_ssh', {
    host,
    user,
    password: password ?? null,
  });
}
