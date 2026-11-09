// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import { SyncContext } from '@/providers/Sync/context';
import type { SyncContextValue } from '@/providers/Sync/context';
import type { Replica } from '@/utils/replica';
import type { RejectedOperation, SyncClient, SyncSnapshot } from '@/utils/sync';
import { SyncSection } from './SyncSection';

afterEach(cleanup);

type RenderOptions = {
  persisted: boolean;
  rejected?: readonly RejectedOperation[];
  clearRejectedOperation?: (operationId: string) => void;
};

function renderSection(options: RenderOptions) {
  const snapshot = {
    confirmed: { projects: [] },
    visible: { projects: [] },
    outbox: [],
    needsBootstrap: false,
    persisted: options.persisted,
    lastSeenSeq: 0,
  };

  const clearRejectedOperation = options.clearRejectedOperation ?? vi.fn();

  const replicaContext: ReplicaContextValue = {
    status: 'ready',
    replica: {
      getSnapshot: () => snapshot,
      subscribe: () => () => undefined,
    } as unknown as Replica,
    error: undefined,
    retry: () => undefined,
  };

  // useSyncExternalStore compares snapshots by identity, so return the same object.
  const syncSnapshot: SyncSnapshot = {
    status: 'idle',
    error: null,
    rejected: options.rejected ?? [],
    lastSyncedAt: null,
  };
  const client: SyncClient = {
    getSnapshot: () => syncSnapshot,
    subscribe: () => () => undefined,
    sync: async () => undefined,
    clearRejectedOperation,
    clearAllRejectedOperations: () => undefined,
  };
  const syncContext: SyncContextValue = {
    client,
    syncNow: async () => undefined,
    clearRejectedOperation,
    clearAllRejectedOperations: () => undefined,
  };

  const { container } = render(
    <ReplicaContext.Provider value={replicaContext}>
      <SyncContext.Provider value={syncContext}>
        <SyncSection />
      </SyncContext.Provider>
    </ReplicaContext.Provider>,
  );

  return { clearRejectedOperation, container };
}

describe('SyncSection', () => {
  it('warns when local storage is not persistent', () => {
    renderSection({ persisted: false });

    expect(screen.getByText('Local storage is not persistent')).toBeDefined();
  });

  it('shows a rejected operation and dismisses it by id', () => {
    const clearRejectedOperation = vi.fn();
    renderSection({
      persisted: true,
      rejected: [{ operation_id: 'abcdef1234567890', reason: 'conflict' }],
      clearRejectedOperation,
    });

    expect(screen.getByText('A newer change took precedence.')).toBeDefined();
    expect(screen.getByText('abcdef12')).toBeDefined();

    fireEvent.click(screen.getByRole('button', { name: 'Dismiss' }));

    expect(clearRejectedOperation).toHaveBeenCalledWith('abcdef1234567890');
  });

  it('renders nothing when storage is persistent and nothing was rejected', () => {
    const { container } = renderSection({ persisted: true });

    expect(container.firstChild).toBeNull();
  });
});
