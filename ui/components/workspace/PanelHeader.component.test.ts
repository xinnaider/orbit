import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import PanelHeader from './PanelHeader.svelte';

describe('PanelHeader', () => {
  it('renders quiet pane header title, status, and close button', async () => {
    const onClose = vi.fn();
    const { getByText, getByLabelText, container } = render(PanelHeader, {
      props: {
        title: 'Refactor billing flow',
        status: 'running',
        closeLabel: 'Close chat pane',
        onClose,
        focused: true,
      },
    });

    expect(container.querySelector('.panel-header.quiet-pane-header')).toBeTruthy();
    expect(getByText('Refactor billing flow')).toBeTruthy();
    expect(getByText('running')).toBeTruthy();
    await fireEvent.click(getByLabelText('Close chat pane'));
    expect(onClose).toHaveBeenCalledOnce();
  });
});
