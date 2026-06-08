import { describe, it, expect, beforeEach } from 'vitest';
import {
  registerDaemonRun,
  resolveSessionForRun,
  resolveRunForSession,
  clearDaemonSession,
  resetDaemonRuns,
} from './daemon-runs';

describe('daemon run registry', () => {
  beforeEach(() => resetDaemonRuns());

  it('resolves session for a registered run and vice versa', () => {
    registerDaemonRun('run_1', 42);
    expect(resolveSessionForRun('run_1')).toBe(42);
    expect(resolveRunForSession(42)).toBe('run_1');
  });

  it('returns null for unknown run/session', () => {
    expect(resolveSessionForRun('nope')).toBeNull();
    expect(resolveRunForSession(999)).toBeNull();
  });

  it('re-registering a session drops the stale run mapping', () => {
    registerDaemonRun('run_old', 7);
    registerDaemonRun('run_new', 7);
    expect(resolveRunForSession(7)).toBe('run_new');
    expect(resolveSessionForRun('run_new')).toBe(7);
    expect(resolveSessionForRun('run_old')).toBeNull();
  });

  it('clearing a session forgets both directions', () => {
    registerDaemonRun('run_1', 1);
    clearDaemonSession(1);
    expect(resolveSessionForRun('run_1')).toBeNull();
    expect(resolveRunForSession(1)).toBeNull();
  });

  it('keeps independent sessions isolated', () => {
    registerDaemonRun('run_a', 1);
    registerDaemonRun('run_b', 2);
    expect(resolveSessionForRun('run_a')).toBe(1);
    expect(resolveSessionForRun('run_b')).toBe(2);
    clearDaemonSession(1);
    expect(resolveSessionForRun('run_b')).toBe(2);
  });
});
