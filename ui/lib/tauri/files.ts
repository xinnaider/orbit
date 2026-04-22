import { invoke } from './invoke';

export async function readFileContent(path: string): Promise<string> {
  return invoke<string>('read_file_content', { path });
}
