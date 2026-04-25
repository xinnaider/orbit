import { invoke } from './invoke';

export interface ApiKeyCreated {
  id: string;
  label: string;
  key: string;
}

export interface ApiKeyInfo {
  id: string;
  label: string;
  createdAt: string;
}

export interface HttpSettings {
  enabled: boolean;
  host: string;
  port: number;
}

export async function generateApiKey(label: string): Promise<ApiKeyCreated> {
  return invoke<ApiKeyCreated>('generate_api_key', { label });
}

export async function listApiKeys(): Promise<ApiKeyInfo[]> {
  return invoke<ApiKeyInfo[]>('list_api_keys');
}

export async function revokeApiKey(id: string): Promise<boolean> {
  return invoke<boolean>('revoke_api_key', { id });
}

export async function getHttpSettings(): Promise<HttpSettings> {
  return invoke<HttpSettings>('get_http_settings');
}

export async function setHttpSettings(enabled: boolean, host: string, port: number): Promise<void> {
  return invoke<void>('set_http_settings', { enabled, host, port });
}

export async function getLanIp(): Promise<string> {
  return invoke<string>('get_lan_ip');
}
