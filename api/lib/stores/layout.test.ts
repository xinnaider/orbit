import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { splitLayout, openPane, closePane, focusPane, assignSession, clearSession } from './layout';

const reset = () =>
  splitLayout.set({
    panes: { tl: null, tr: null, bl: null, br: null },
    visible: ['tl'],
    focused: 'tl',
  });

beforeEach(reset);

// ── assignSession ─────────────────────────────────────────────

describe('assignSession', () => {
  it('puts sessionId in the given pane', () => {
    assignSession('tl', 42);
    expect(get(splitLayout).panes.tl).toBe(42);
  });

  it('focuses the assigned pane', () => {
    openPane('tr', 1);
    assignSession('tl', 42);
    expect(get(splitLayout).focused).toBe('tl');
  });

  it('does not add pane to visible list if already visible', () => {
    assignSession('tl', 42);
    expect(get(splitLayout).visible).toEqual(['tl']);
  });
});

// ── openPane ──────────────────────────────────────────────────

describe('openPane', () => {
  it('adds pane to visible list when not present', () => {
    openPane('tr', 10);
    const l = get(splitLayout);
    expect(l.visible).toContain('tr');
    expect(l.panes.tr).toBe(10);
    expect(l.focused).toBe('tr');
  });

  it('does not duplicate pane in visible list', () => {
    openPane('tl', 5);
    openPane('tl', 7);
    const visible = get(splitLayout).visible.filter((p) => p === 'tl');
    expect(visible).toHaveLength(1);
  });

  it('updates session when pane already visible', () => {
    openPane('tl', 5);
    openPane('tl', 9);
    expect(get(splitLayout).panes.tl).toBe(9);
  });
});

// ── closePane ─────────────────────────────────────────────────

describe('closePane', () => {
  it('removes pane from visible list', () => {
    openPane('tr', 10);
    closePane('tr');
    expect(get(splitLayout).visible).not.toContain('tr');
  });

  it('shifts focus to first remaining pane when focused pane is closed', () => {
    openPane('tr', 10);
    focusPane('tr');
    closePane('tr');
    expect(get(splitLayout).focused).toBe('tl');
  });

  it('does not close the last remaining pane', () => {
    closePane('tl');
    expect(get(splitLayout).visible).toContain('tl');
  });

  it('keeps focus unchanged when non-focused pane is closed', () => {
    openPane('tr', 10);
    focusPane('tl');
    closePane('tr');
    expect(get(splitLayout).focused).toBe('tl');
  });

  it('sets pane sessionId to null when closed', () => {
    openPane('tr', 10);
    closePane('tr');
    expect(get(splitLayout).panes.tr).toBeNull();
  });

  it('collapses to single pane when all remaining panes are empty', () => {
    // tl has session, tr is empty (as after splitPane)
    openPane('tr', 0); // open tr empty — use 0 as placeholder then clear
    assignSession('tl', 42);
    // manually set tr to null to simulate the "split but not assigned" case
    splitLayout.update((l) => ({ ...l, panes: { ...l.panes, tr: null } }));
    // now close tl (the one with the session)
    closePane('tl');
    const l = get(splitLayout);
    expect(l.visible).toHaveLength(1);
    expect(l.visible[0]).toBe('tr');
  });
});

// ── focusPane ─────────────────────────────────────────────────

describe('focusPane', () => {
  it('changes the focused pane', () => {
    openPane('tr', 1);
    focusPane('tr');
    expect(get(splitLayout).focused).toBe('tr');
  });

  it('does nothing when pane is not visible', () => {
    focusPane('tr'); // tr is not in visible (['tl'] only)
    expect(get(splitLayout).focused).toBe('tl');
  });
});

// ── clearSession ──────────────────────────────────────────────

describe('clearSession', () => {
  it('nulls out every pane that held the session', () => {
    assignSession('tl', 42);
    openPane('tr', 42);
    clearSession(42);
    const { panes } = get(splitLayout);
    expect(panes.tl).toBeNull();
    expect(panes.tr).toBeNull();
  });

  it('does not affect panes with different sessions', () => {
    assignSession('tl', 1);
    openPane('tr', 2);
    clearSession(1);
    expect(get(splitLayout).panes.tr).toBe(2);
  });
});
