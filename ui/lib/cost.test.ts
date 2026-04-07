import { describe, it, expect } from 'vitest';
import { estimateCost, formatCost, formatTokens } from './cost';
import type { TokenUsage } from './types';

function tokens(overrides: Partial<TokenUsage> = {}): TokenUsage {
  return { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, ...overrides };
}

// ── estimateCost ──────────────────────────────────────────────

describe('estimateCost', () => {
  it('returns 0 for zero tokens', () => {
    expect(estimateCost(tokens(), 'claude-sonnet-4-6')).toBe(0);
  });

  it('calculates sonnet input cost correctly', () => {
    // 1M input tokens at $3/M = $3
    const cost = estimateCost(tokens({ input: 1_000_000 }), 'claude-sonnet-4-6');
    expect(cost).toBeCloseTo(3, 5);
  });

  it('calculates opus output cost correctly', () => {
    // 1M output tokens at $75/M = $75
    const cost = estimateCost(tokens({ output: 1_000_000 }), 'claude-opus-4-6');
    expect(cost).toBeCloseTo(75, 5);
  });

  it('calculates haiku cost correctly', () => {
    // 1M input at $0.8 + 500K output at $4/M * 0.5 = 0.8 + 2 = $2.8
    const cost = estimateCost(
      tokens({ input: 1_000_000, output: 500_000 }),
      'claude-haiku-4-5-20251001'
    );
    expect(cost).toBeCloseTo(2.8, 5);
  });

  it('uses default pricing for unknown model', () => {
    // default = sonnet pricing: 1M input = $3
    const cost = estimateCost(tokens({ input: 1_000_000 }), 'unknown-model');
    expect(cost).toBeCloseTo(3, 5);
  });

  it('uses default pricing for null model', () => {
    const cost = estimateCost(tokens({ input: 1_000_000 }), null);
    expect(cost).toBeCloseTo(3, 5);
  });

  it('includes cache read and write costs', () => {
    // sonnet: cacheRead $0.3/M, cacheWrite $3.75/M
    const cost = estimateCost(
      tokens({ cacheRead: 1_000_000, cacheWrite: 1_000_000 }),
      'claude-sonnet-4-6'
    );
    expect(cost).toBeCloseTo(0.3 + 3.75, 5);
  });
});

// ── formatCost ────────────────────────────────────────────────

describe('formatCost', () => {
  it('returns <$0.01 for very small costs', () => {
    expect(formatCost(0)).toBe('<$0.01');
    expect(formatCost(0.005)).toBe('<$0.01');
    expect(formatCost(0.009)).toBe('<$0.01');
  });

  it('formats costs >= $0.01 with 2 decimal places', () => {
    expect(formatCost(0.01)).toBe('$0.01');
    expect(formatCost(1.5)).toBe('$1.50');
    expect(formatCost(12.345)).toBe('$12.35');
  });
});

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
