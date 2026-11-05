// @vitest-environment jsdom
import 'fake-indexeddb/auto';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ReplicaProvider, useReplicaStatus } from '@/providers/Replica';
import { openReplica } from '@/utils/replica';
import type { SyncClient, SyncSnapshot } from '@/utils/sync';
import { useSync, useSyncSnapshot } from './hook';
import { SyncProvider } from './provider';

// useSyncExternalStore compares snapshots by identity, so the fake client must
// return the same object rather than building a new one on each call.
const IDLE_SNAPSHOT: SyncSnapshot = { status: 'idle', error: null, rejected: [] };

function createClient(sync: SyncClient['sync']): SyncClient {
  return {
    sync,
    getSnapshot: () => IDLE_SNAPSHOT,
    subscribe: () => () => undefined,
  };
}

type ReadyGateProps = {
  children: React.ReactNode;
};

function ReadyGate(props: ReadyGateProps) {
  const { children } = props;
  const { status } = useReplicaStatus();
  // useReplica throws until the replica is ready, so hold the provider back
  return status === 'ready' ? children : null;
}

function Probe() {
  const { syncNow } = useSync();
  const snapshot = useSyncSnapshot();

  function handleManual() {
    void syncNow();
  }

  return (
    <div>
      <span data-testid="status">{snapshot.status}</span>
      <button onClick={handleManual}>manual</button>
    </div>
  );
}

let name = '';

beforeEach(() => {
  name = `scry-sync-provider-test-${crypto.randomUUID()}`;
});

afterEach(async () => {
  cleanup();
  await deleteDatabase(name);
});

function renderApp(client: SyncClient) {
  return render(
    <ReplicaProvider open={() => openReplica({ databaseName: name })}>
      <ReadyGate>
        <SyncProvider client={client}>
          <Probe />
        </SyncProvider>
      </ReadyGate>
    </ReplicaProvider>,
  );
}

describe('SyncProvider', () => {
  it('runs the startup sync', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    renderApp(client);

    await waitFor(() => {
      expect(sync).toHaveBeenCalledTimes(1);
    });
    expect(screen.getByTestId('status').textContent).toBe('idle');
  });

  it('manual sync calls the client', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    renderApp(client);

    await waitFor(() => {
      expect(sync).toHaveBeenCalledTimes(1);
    });

    fireEvent.click(screen.getByRole('button', { name: 'manual' }));

    await waitFor(() => {
      expect(sync).toHaveBeenCalledTimes(2);
    });
  });

  it('closes cleanly', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    const { unmount } = renderApp(client);

    await waitFor(() => {
      expect(sync).toHaveBeenCalledTimes(1);
    });

    unmount();
  });
});

function deleteDatabase(databaseName: string): Promise<void> {
  return new Promise((resolve) => {
    const request = indexedDB.deleteDatabase(databaseName);
    request.onsuccess = () => resolve();
    request.onerror = () => resolve();
    request.onblocked = () => resolve();
  });
}
