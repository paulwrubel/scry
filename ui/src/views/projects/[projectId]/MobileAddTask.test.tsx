// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MobileAddTask } from './MobileAddTask';

function renderTask(onCreate: (title: string) => Promise<void>) {
  render(<MobileAddTask onCreate={onCreate} />);
}

afterEach(cleanup);

describe('MobileAddTask', () => {
  it('opens the sheet and adds a task without closing', async () => {
    const onCreate = vi.fn(async (_title: string) => undefined);
    renderTask(onCreate);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText('Task title'), {
      target: { value: '  Mobile task  ' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Add' }));

    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('Mobile task');
    });
    expect((screen.getByLabelText('Task title') as HTMLInputElement).value).toBe('');
    expect(screen.getByRole('button', { name: 'Add' })).toBeDefined();
  });

  it('shows an error and keeps the sheet open when creation fails', async () => {
    const onCreate = vi.fn(async (_title: string) => {
      throw new Error('offline');
    });
    renderTask(onCreate);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText('Task title'), { target: { value: 'Task' } });
    fireEvent.click(screen.getByRole('button', { name: 'Add' }));

    expect(await screen.findByText('Could not add this task.')).toBeDefined();
    expect(screen.getByLabelText('Task title')).toBeDefined();
  });

  it('closes the sheet on Escape', () => {
    renderTask(vi.fn(async () => undefined));

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText('Task title'), { target: { value: 'Task' } });

    fireEvent.keyDown(document, { key: 'Escape' });

    expect(screen.queryByRole('button', { name: 'Add' })).toBeNull();
  });
});
