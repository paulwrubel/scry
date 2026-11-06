// @vitest-environment jsdom
import 'fake-indexeddb/auto';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AuthContext } from '@/providers/Auth/context';
import type { AuthContextType } from '@/providers/Auth/context';
import { ReplicaProvider, useReplicaStatus } from '@/providers/Replica';
import { openReplica } from '@/utils/replica';
import type { SyncClient, SyncSnapshot } from '@/utils/sync';
import { useSync, useSyncSnapshot } from './hook';
import { SyncProvider } from './provider';

// useSyncExternalStore compares snapshots by identity, so the fake client must
// return the same object rather than building a new one on each call.
const IDLE_SNAPSHOT: SyncSnapshot = {
  status: 'idle',
  error: null,
  rejected: [],
  lastSyncedAt: null,
};

const AUTH_VALUE: AuthContextType = {
  isAuthenticated: true,
  isAuthLoading: false,
  login: async () => ({ ok: true }),
  logout: async () => undefined,
};

const SIGNED_OUT_AUTH: AuthContextType = { ...AUTH_VALUE, isAuthenticated: false };

function createClient(sync: SyncClient['sync']): SyncClient {
  return {
    sync,
    getSnapshot: () => IDLE_SNAPSHOT,
    subscribe: () => () => undefined,
    clearRejectedOperation: () => undefined,
    clearAllRejectedOperations: () => undefined,
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

function renderApp(client: SyncClient, auth: AuthContextType = AUTH_VALUE) {
  return render(
    <AuthContext.Provider value={auth}>
      <ReplicaProvider open={() => openReplica({ databaseName: name })}>
        <ReadyGate>
          <SyncProvider client={client}>
            <Probe />
          </SyncProvider>
        </ReadyGate>
      </ReplicaProvider>
    </AuthContext.Provider>,
  );
}

describe('SyncProvider', () => {
  it('runs the startup sync', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    renderApp(client);

    await waitFor(() => {
      expect(sync).toHaveBeenCalled();
    });
    expect(screen.getByTestId('status').textContent).toBe('idle');
  });

  it('manual sync calls the client', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    renderApp(client);

    await waitFor(() => {
      expect(sync).toHaveBeenCalled();
    });

    const callsAfterStartup = sync.mock.calls.length;
    fireEvent.click(screen.getByRole('button', { name: 'manual' }));

    await waitFor(() => {
      expect(sync.mock.calls.length).toBeGreaterThan(callsAfterStartup);
    });
  });

  it('closes cleanly', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    const { unmount } = renderApp(client);

    await waitFor(() => {
      expect(sync).toHaveBeenCalled();
    });

    unmount();
  });

  it('does not sync while signed out', () => {
    const sync = vi.fn().mockResolvedValue(undefined);

    renderApp(createClient(sync), SIGNED_OUT_AUTH);

    expect(sync).not.toHaveBeenCalled();
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
