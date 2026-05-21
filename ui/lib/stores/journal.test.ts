import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { pendingMessages } from './journal';

describe('pendingMessages store', () => {
  beforeEach(() => {
    pendingMessages.clear();
  });

  it('starts empty', () => {
    expect(get(pendingMessages)).toEqual([]);
  });

  it('add appends a message', () => {
    pendingMessages.add('hello');
    const msgs = get(pendingMessages);
    expect(msgs).toHaveLength(1);
    expect(msgs[0].text).toBe('hello');
  });

  it('add increments id', () => {
    pendingMessages.add('first');
    pendingMessages.add('second');
    const msgs = get(pendingMessages);
    expect(msgs[0].id).not.toBe(msgs[1].id);
  });

  it('clear removes all', () => {
    pendingMessages.add('a');
    pendingMessages.add('b');
    pendingMessages.add('c');
    expect(get(pendingMessages)).toHaveLength(3);

    pendingMessages.clear();
    expect(get(pendingMessages)).toEqual([]);
  });

  it('remove deletes by id', () => {
    pendingMessages.add('hello');
    const msgs = get(pendingMessages);
    const id = msgs[0].id;

    pendingMessages.remove(id);
    expect(get(pendingMessages)).toEqual([]);
  });

  it('multiple adds keep order', () => {
    pendingMessages.add('first');
    pendingMessages.add('second');
    const msgs = get(pendingMessages);
    expect(msgs[0].text).toBe('first');
    expect(msgs[1].text).toBe('second');
  });
});
