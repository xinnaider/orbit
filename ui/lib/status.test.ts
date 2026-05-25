import { describe, expect, it } from 'vitest';
import { sessionStatusDotColor, statusColor } from './status';

describe('sessionStatusDotColor', () => {
  it('uses live status when attention reason is null', () => {
    expect(
      sessionStatusDotColor('waiting', { requiresAttention: true, reason: null })
    ).toBe(statusColor('waiting'));
  });

  it('uses live status when attention is missing', () => {
    expect(sessionStatusDotColor('idle', null)).toBe(statusColor('idle'));
  });

  it('does not default to accent green for unknown attention', () => {
    expect(sessionStatusDotColor('waiting', { requiresAttention: true, reason: 'other' })).toBe(
      statusColor('waiting')
    );
  });

  it('highlights permission with input color', () => {
    expect(
      sessionStatusDotColor('waiting', { requiresAttention: true, reason: 'permission' })
    ).toBe('var(--s-input)');
  });
});
