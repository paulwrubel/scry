// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import { NewTaskForm } from './NewTaskForm';

afterEach(cleanup);

const inputLabel = 'Task title or GitHub URL';

describe('NewTaskForm', () => {
  it('adds a trimmed task and keeps the form open', async () => {
    const onCreate = vi.fn(async (_title: string) => undefined);
    render(<NewTaskForm onCreate={onCreate} />);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText(inputLabel), {
      target: { value: '  New task  ' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Add' }));

    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('New task', null);
    });
    expect((screen.getByLabelText(inputLabel) as HTMLInputElement).value).toBe('');
  });

  it('creates a placeholder task from a GitHub URL', async () => {
    const onCreate = vi.fn(async (_title: string, _identity: ExternalIdentity | null) => undefined);
    render(<NewTaskForm onCreate={onCreate} />);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText(inputLabel), {
      target: { value: 'https://github.com/acme/widgets/pull/123/files' },
    });
    const add = screen.getByRole('button', { name: 'Add' }) as HTMLButtonElement;
    await waitFor(() => {
      expect(add.disabled).toBe(false);
    });
    fireEvent.click(add);

    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('acme/widgets#123', {
        provider: 'github',
        owner: 'acme',
        repo: 'widgets',
        number: 123,
      });
      expect((screen.getByLabelText(inputLabel) as HTMLInputElement).value).toBe('');
    });
  });

  it('disables Add for a non-GitHub URL', async () => {
    render(<NewTaskForm onCreate={vi.fn(async () => undefined)} />);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText(inputLabel), {
      target: { value: 'https://gitlab.com/acme/widgets/issues/123' },
    });

    expect(
      await screen.findByText('Only GitHub issue or pull request URLs are supported.'),
    ).toBeDefined();
    expect((screen.getByRole('button', { name: 'Add' }) as HTMLButtonElement).disabled).toBe(true);
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
    fireEvent.change(screen.getByLabelText(inputLabel), {
      target: { value: 'Discard me' },
    });
    fireEvent.pointerDown(document.body);

    expect(screen.queryByRole('button', { name: 'Add' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Add task' })).toBeDefined();
  });

  it('does not cancel on a pointer-down inside the form', () => {
    render(<NewTaskForm onCreate={vi.fn(async () => undefined)} />);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.pointerDown(screen.getByLabelText(inputLabel));

    expect(screen.getByRole('button', { name: 'Add' })).toBeDefined();
  });
});
