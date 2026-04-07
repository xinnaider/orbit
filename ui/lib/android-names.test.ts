import { describe, it, expect } from 'vitest';
import { generateAgentName } from './android-names';

describe('generateAgentName', () => {
  it('retorna apenas o codename, sem projeto', () => {
    const name = generateAgentName();
    expect(name).not.toContain('·');
    expect(name).toMatch(/^[a-z]+$/);
  });

  it('gera resultados variados (probabilístico com 20 amostras)', () => {
    const names = new Set(Array.from({ length: 20 }, () => generateAgentName()));
    expect(names.size).toBeGreaterThan(1);
  });
});
