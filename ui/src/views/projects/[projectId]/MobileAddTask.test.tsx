// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MobileAddTask } from './MobileAddTask';

function renderTask(onCreate: (title: string) => Promise<void>) {
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
