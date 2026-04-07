import { describe, it, expect } from 'vitest';
import { generateSessionName } from './android-names';

describe('generateSessionName', () => {
  it('inclui o nome do projeto na saída', () => {
    const name = generateSessionName('my-project');
    expect(name).toContain('my-project');
  });

  it('segue o padrão "<codename> · <project>"', () => {
    const name = generateSessionName('orbit');
    expect(name).toMatch(/^[a-z]+ · orbit$/);
  });

  it('gera resultados variados (probabilístico com 20 amostras)', () => {
    const names = new Set(Array.from({ length: 20 }, () => generateSessionName('p')));
    expect(names.size).toBeGreaterThan(1);
  });
});
