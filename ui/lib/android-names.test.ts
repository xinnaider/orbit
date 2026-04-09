import { describe, it, expect } from 'vitest';
import { generateAgentName, parseSessionName } from './android-names';

describe('generateAgentName', () => {
  it('returns only the codename, no project', () => {
    const name = generateAgentName();
    expect(name).not.toContain('·');
    expect(name).toMatch(/^[a-z]+$/);
  });

  it('generates varied results (probabilistic with 20 samples)', () => {
    const names = new Set(Array.from({ length: 20 }, () => generateAgentName()));
    expect(names.size).toBeGreaterThan(1);
  });
});

describe('parseSessionName', () => {
  it('splits prefix and suffix on " · "', () => {
    expect(parseSessionName('hammerhead · orbit-dashboard')).toEqual({
      prefix: 'hammerhead',
      suffix: 'orbit-dashboard',
    });
  });

  it('returns empty suffix when no separator', () => {
    expect(parseSessionName('hammerhead')).toEqual({ prefix: 'hammerhead', suffix: '' });
  });

  it('handles null/undefined name', () => {
    expect(parseSessionName(null)).toEqual({ prefix: '', suffix: '' });
    expect(parseSessionName(undefined)).toEqual({ prefix: '', suffix: '' });
  });

  it('trims whitespace from both parts', () => {
    expect(parseSessionName('  tokay  ·  my-project  ')).toEqual({
      prefix: 'tokay',
      suffix: 'my-project',
    });
  });
});
