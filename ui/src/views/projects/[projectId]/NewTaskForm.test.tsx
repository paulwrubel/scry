// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { NewTaskForm } from './NewTaskForm';

afterEach(cleanup);

describe('NewTaskForm', () => {
  it('adds a trimmed task and keeps the form open', async () => {
    const onCreate = vi.fn(async (_title: string) => undefined);
    render(<NewTaskForm onCreate={onCreate} />);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText('Task title'), {
      target: { value: '  New task  ' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Add' }));

    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('New task');
    });
    expect((screen.getByLabelText('Task title') as HTMLInputElement).value).toBe('');
  });

  it('puts Cancel before Add in the footer', () => {
    render(<NewTaskForm onCreate={vi.fn(async () => undefined)} />);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));

    const cancel = screen.getByRole('button', { name: 'Cancel' });
    const add = screen.getByRole('button', { name: 'Add' });
    expect(cancel.compareDocumentPosition(add) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('cancels on a pointer-down outside the form', () => {
    render(<NewTaskForm onCreate={vi.fn(async () => undefined)} />);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText('Task title'), {
      target: { value: 'Discard me' },
    });
    fireEvent.pointerDown(document.body);

    expect(screen.queryByRole('button', { name: 'Add' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Add task' })).toBeDefined();
  });

  it('does not cancel on a pointer-down inside the form', () => {
    render(<NewTaskForm onCreate={vi.fn(async () => undefined)} />);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.pointerDown(screen.getByLabelText('Task title'));

    expect(screen.getByRole('button', { name: 'Add' })).toBeDefined();
  });
});
