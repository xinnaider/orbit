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
        onClose,
        focused: true,
      },
    });

    expect(container.querySelector('header.topbar.quiet-topbar')).toBeTruthy();
    expect(container.querySelector('header.glass-topbar')).toBeFalsy();
    expect(getByText('Refactor billing flow')).toBeTruthy();
    expect(getByText('running')).toBeTruthy();
    await fireEvent.click(getByLabelText('Close panel'));
    expect(onClose).toHaveBeenCalledOnce();
  });
});
