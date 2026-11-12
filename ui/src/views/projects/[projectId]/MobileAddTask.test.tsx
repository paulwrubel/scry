// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import { MobileAddTask } from './MobileAddTask';

const inputLabel = 'Task title or GitHub URL';

function renderTask(
  onCreate: (title: string, externalIdentity: ExternalIdentity | null) => Promise<void>,
) {
  render(<MobileAddTask onCreate={onCreate} />);
}

const originalVisualViewport = Object.getOwnPropertyDescriptor(window, 'visualViewport');

afterEach(() => {
  cleanup();
  if (originalVisualViewport === undefined) {
    Reflect.deleteProperty(window, 'visualViewport');
  } else {
    Object.defineProperty(window, 'visualViewport', originalVisualViewport);
  }
});

describe('MobileAddTask', () => {
  it('opens the sheet and adds a task without closing', async () => {
    const onCreate = vi.fn(async (_title: string, _identity: ExternalIdentity | null) => undefined);
    renderTask(onCreate);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText(inputLabel), {
      target: { value: '  Mobile task  ' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Add' }));

    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('Mobile task', null);
    });
    // Testing Library returns HTMLElement, but the accessible label identifies a text input
    expect((screen.getByLabelText(inputLabel) as HTMLInputElement).value).toBe('');
    expect(screen.getByRole('button', { name: 'Add' })).toBeDefined();
  });

  it('creates a placeholder task from a GitHub URL', async () => {
    const onCreate = vi.fn(async (_title: string, _identity: ExternalIdentity | null) => undefined);
    renderTask(onCreate);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText(inputLabel), {
      target: { value: 'https://github.com/acme/widgets/issues/123' },
    });
    const add = screen.getByRole('button', { name: 'Add' });
    await waitFor(() => {
      expect(add.hasAttribute('disabled')).toBe(false);
    });
    fireEvent.click(add);

    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('acme/widgets#123', {
        provider: 'github',
        owner: 'acme',
        repo: 'widgets',
        number: 123,
      });
    });
  });

  it('shows an error and keeps the sheet open when creation fails', async () => {
    const onCreate = vi.fn(async (_title: string, _identity: ExternalIdentity | null) => {
      throw new Error('offline');
    });
    renderTask(onCreate);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText(inputLabel), { target: { value: 'Task' } });
    fireEvent.click(screen.getByRole('button', { name: 'Add' }));

    expect(await screen.findByText('Could not add this task.')).toBeDefined();
    expect(screen.getByLabelText(inputLabel)).toBeDefined();
  });

  it('closes the sheet on Escape', () => {
    renderTask(vi.fn(async () => undefined));

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText(inputLabel), { target: { value: 'Task' } });

    fireEvent.keyDown(document, { key: 'Escape' });

    expect(screen.queryByRole('button', { name: 'Add' })).toBeNull();
  });

  it('lifts the sheet by the visual viewport inset', () => {
    const viewport = new EventTarget() as unknown as VisualViewport;
    Object.defineProperty(viewport, 'height', {
      configurable: true,
      value: window.innerHeight - 300,
    });
    Object.defineProperty(viewport, 'offsetTop', { configurable: true, value: 0 });
    Object.defineProperty(window, 'visualViewport', { configurable: true, value: viewport });

    renderTask(vi.fn(async () => undefined));
    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));

    expect(screen.getByTestId('flowbite-drawer').style.bottom).toBe('300px');

    act(() => {
      Object.defineProperty(viewport, 'height', {
        configurable: true,
        value: window.innerHeight - 400,
      });
      viewport.dispatchEvent(new Event('resize'));
    });

    expect(screen.getByTestId('flowbite-drawer').style.bottom).toBe('400px');
  });
});
