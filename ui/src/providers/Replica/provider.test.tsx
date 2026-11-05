// @vitest-environment jsdom
import 'fake-indexeddb/auto';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { openReplica } from '@/utils/replica';
import type { Replica, ReplicaSnapshot } from '@/utils/replica';
import { useReplica, useReplicaSnapshot, useReplicaStatus } from './hook';
import { ReplicaProvider } from './provider';

type ReadyGateProps = {
  children: React.ReactNode;
};

function ReadyGate(props: ReadyGateProps) {
  const { children } = props;
  const { status } = useReplicaStatus();
  // useReplica throws until the replica is ready, so hold the probe back
  return status === 'ready' ? children : null;
}

function StatusProbe() {
  const { status, error } = useReplicaStatus();
  return (
    <div>
      <span data-testid="status">{status}</span>
      {error ? <span data-testid="error">{error.message}</span> : null}
    </div>
  );
}

function SnapshotProbe() {
  const snapshot = useReplicaSnapshot();
  const replica = useReplica();
  const addProject = () => {
    void replica.enqueue({
      operation_id: 'op-1',
      operation_version: 1,
      kind: 'create_project',
      id: 'project-1',
      name: 'Project',
      description: null,
      task_sorting_mode: 'manual',
      show_priority: false,
      created_at: '2026-01-01T00:00:00Z',
    });
  };

  return (
    <div>
      <span data-testid="projects">{snapshot.visible.projects.length}</span>
      <button onClick={addProject}>add</button>
    </div>
  );
}

function RetryProbe() {
  const { retry } = useReplicaStatus();
  return <button onClick={retry}>retry</button>;
}

let name = '';

beforeEach(() => {
  name = `scry-replica-provider-test-${crypto.randomUUID()}`;
});

afterEach(async () => {
  cleanup();
  await deleteDatabase(name);
});

describe('ReplicaProvider', () => {
  it('renders loading then ready', async () => {
    const pending = deferred<Replica>();

    render(
      <ReplicaProvider open={() => pending.promise}>
        <StatusProbe />
      </ReplicaProvider>,
    );

    expect(screen.getByTestId('status').textContent).toBe('loading');

    pending.resolve(await openReplica({ databaseName: name }));

    await waitFor(() => {
      expect(screen.getByTestId('status').textContent).toBe('ready');
    });
  });

  it('re-renders when the Replica snapshot changes', async () => {
    render(
      <ReplicaProvider open={() => openReplica({ databaseName: name })}>
        <ReadyGate>
          <SnapshotProbe />
        </ReadyGate>
      </ReplicaProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('projects').textContent).toBe('0');
    });

    fireEvent.click(screen.getByRole('button', { name: 'add' }));

    await waitFor(() => {
      expect(screen.getByTestId('projects').textContent).toBe('1');
    });
  });

  it('renders an error and retries', async () => {
    const replica = await openReplica({ databaseName: name });
    const open = vi
      .fn<() => Promise<Replica>>()
      .mockRejectedValueOnce(new Error('failed to open'))
      .mockResolvedValue(replica);

    render(
      <ReplicaProvider open={open}>
        <StatusProbe />
        <RetryProbe />
      </ReplicaProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('error').textContent).toBe('failed to open');
    });

    fireEvent.click(screen.getByRole('button', { name: 'retry' }));

    await waitFor(() => {
      expect(screen.getByTestId('status').textContent).toBe('ready');
    });
  });

  it('closes the Replica on unmount', async () => {
    const snapshot: ReplicaSnapshot = {
      confirmed: { projects: [] },
      visible: { projects: [] },
      outbox: [],
      needsBootstrap: false,
      persisted: false,
    };
    const close = vi.fn();
    const replica = {
      getSnapshot: () => snapshot,
      subscribe: () => () => undefined,
      close,
    } as unknown as Replica;

    const { unmount } = render(
      <ReplicaProvider open={() => Promise.resolve(replica)}>
        <StatusProbe />
      </ReplicaProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('status').textContent).toBe('ready');
    });

    unmount();

    expect(close).toHaveBeenCalledTimes(1);
  });
});

function deferred<T>() {
  let resolve: (value: T) => void = () => undefined;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

function deleteDatabase(databaseName: string): Promise<void> {
  return new Promise((resolve) => {
    const request = indexedDB.deleteDatabase(databaseName);
    request.onsuccess = () => resolve();
    request.onerror = () => resolve();
    request.onblocked = () => resolve();
  });
}
