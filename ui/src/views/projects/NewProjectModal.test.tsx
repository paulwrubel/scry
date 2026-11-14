// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Operation } from '@/generated/Operation';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import { NewProjectModal } from './NewProjectModal';
import { createReplicaMock, makeReplicaSnapshot } from '@/test/fixtures';

class FakeCloseWatcher extends EventTarget {
  static instances: FakeCloseWatcher[] = [];

  constructor() {
    super();
    FakeCloseWatcher.instances.push(this);
  }

  requestClose() {
    this.close();
  }

  close() {
    this.dispatchEvent(new Event('close'));
  }

  destroy() {
    FakeCloseWatcher.instances = FakeCloseWatcher.instances.filter((instance) => instance !== this);
  }
}

function renderModal(
  enqueue: (operation: Operation) => Promise<void>,
  existingNames: readonly string[] = [],
  onCreated = vi.fn(),
  onClose = vi.fn(),
) {
  const snapshot = makeReplicaSnapshot([]);

  const context: ReplicaContextValue = {
    status: 'ready',
    replica: createReplicaMock(snapshot, { enqueue }),
    error: undefined,
    retry: () => undefined,
  };

  render(
    <ReplicaContext.Provider value={context}>
      <NewProjectModal existingNames={existingNames} onClose={onClose} onCreated={onCreated} />
    </ReplicaContext.Provider>,
  );

  return { onCreated, onClose };
}

function createButton() {
  return screen.getByRole('button', { name: 'Create' }) as HTMLButtonElement;
}

afterEach(cleanup);

describe('NewProjectModal', () => {
  it('disables Create until the name is valid', () => {
    renderModal(vi.fn(async () => undefined));

    expect(createButton().disabled).toBe(true);
  });

  it('shows a required-name error', async () => {
    renderModal(vi.fn(async () => undefined));

    fireEvent.change(screen.getByLabelText('Name'), { target: { value: '   ' } });

    expect(await screen.findByText('A project needs a name.')).toBeDefined();
    expect(createButton().disabled).toBe(true);
  });

  it('rejects a duplicate name against existing projects', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderModal(enqueue, ['Alpha']);

    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Alpha' } });

    expect(await screen.findByText('A project with this name already exists.')).toBeDefined();
    expect(createButton().disabled).toBe(true);
    fireEvent.click(createButton());
    expect(enqueue).not.toHaveBeenCalled();
  });

  it('enqueues a create_project operation and reports the new id', async () => {
    const enqueue = vi.fn(async (_operation: Operation) => undefined);
    const { onCreated } = renderModal(enqueue);

    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'New project' } });
    fireEvent.change(screen.getByLabelText('Description'), { target: { value: ' Details ' } });

    await waitFor(() => {
      expect(createButton().disabled).toBe(false);
    });
    fireEvent.click(createButton());

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledTimes(1);
    });

    const operation = enqueue.mock.calls[0][0];
    expect(operation).toMatchObject({
      kind: 'create_project',
      name: 'New project',
      description: 'Details',
    });
    await waitFor(() => {
      expect(onCreated).toHaveBeenCalledWith(operation.id);
    });
  });

  it('shows an error and does not report creation when the enqueue fails', async () => {
    const enqueue = vi.fn(async (_operation: Operation) => {
      throw new Error('offline');
    });
    const { onCreated } = renderModal(enqueue);

    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'New project' } });

    await waitFor(() => {
      expect(createButton().disabled).toBe(false);
    });
    fireEvent.click(createButton());

    expect(await screen.findByText('Could not create this project.')).toBeDefined();
    expect(onCreated).not.toHaveBeenCalled();
    expect(createButton().disabled).toBe(true);

    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Renamed project' } });

    await waitFor(() => {
      expect(screen.queryByText('Could not create this project.')).toBeNull();
    });
    await waitFor(() => {
      expect(createButton().disabled).toBe(false);
    });
  });

  describe('platform close requests', () => {
    beforeEach(() => {
      FakeCloseWatcher.instances = [];
      Object.defineProperty(globalThis, 'CloseWatcher', {
        configurable: true,
        writable: true,
        value: FakeCloseWatcher,
      });
    });

    afterEach(() => {
      Reflect.deleteProperty(globalThis, 'CloseWatcher');
    });

    it('closes on a platform close request', () => {
      const { onClose } = renderModal(vi.fn(async () => undefined));
      const [watcher] = FakeCloseWatcher.instances;
      if (watcher === undefined) {
        throw new Error('expected a CloseWatcher to be created');
      }

      act(() => {
        watcher.close();
      });

      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });
});
