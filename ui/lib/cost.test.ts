import { describe, it, expect } from 'vitest';
import { formatTokens } from './cost';

// ── formatTokens ──────────────────────────────────────────────

describe('formatTokens', () => {
  it('formats millions', () => {
    expect(formatTokens(1_000_000)).toBe('1.0M');
    expect(formatTokens(1_500_000)).toBe('1.5M');
  });

  it('formats thousands', () => {
    expect(formatTokens(1_000)).toBe('1K');
    expect(formatTokens(150_000)).toBe('150K');
  });

  it('formats small numbers as plain string', () => {
    expect(formatTokens(0)).toBe('0');
    expect(formatTokens(500)).toBe('500');
  });
});
