import type { OperationAcknowledgment } from '@/generated/OperationAcknowledgment';
import type { Snapshot } from '@/generated/Snapshot';
import type { SyncResponse } from '@/generated/SyncResponse';
import { getAPIURL, getSnapshot, postSync } from '@/utils/api';
import { isReplicaError } from '@/utils/replica';
import { loadSyncState, saveSyncState } from './persistence';
import { isSyncError, syncError } from './types';
import type {
  RejectedOperation,
  SyncClient,
  SyncClientOptions,
  SyncError,
  SyncSnapshot,
  SyncStatus,
} from './types';

const OPERATION_VERSION = 1;

export function createSyncClient(options: SyncClientOptions): SyncClient {
  const { replica, fetcher = fetch, baseUrl = getAPIURL() } = options;

  const persisted = loadSyncState();
  let status: SyncStatus = 'idle';
  let error: SyncError | null = null;
  let rejected: readonly RejectedOperation[] = persisted.rejected;
  let lastSyncedAt: string | null = persisted.lastSyncedAt;
  let snapshot: SyncSnapshot = { status, error, rejected, lastSyncedAt };
  const listeners = new Set<() => void>();
  let inFlight: Promise<void> | null = null;

  function notify(): void {
    snapshot = { status, error, rejected, lastSyncedAt };
    for (const listener of [...listeners]) {
      listener();
    }
  }

  function persist(): void {
    saveSyncState({ lastSyncedAt, rejected: [...rejected] });
  }

  function mergeRejected(incoming: readonly RejectedOperation[]): void {
    const byId = new Map(rejected.map((item) => [item.operation_id, item]));
    for (const item of incoming) {
      byId.set(item.operation_id, item);
    }
    rejected = [...byId.values()];
    persist();
  }

  // fetch rejects only on transport failures; tag those as network so the
  // handler can distinguish them from malformed responses.
  async function send<T>(request: () => Promise<T>): Promise<T> {
    try {
      return await request();
    } catch (cause) {
      throw syncError('network', 'network request failed', cause);
    }
  }

  async function syncInternal(): Promise<void> {
    status = 'syncing';
    error = null;
    notify();

    try {
      await replica.reload();

      if (replica.needsBootstrap) {
        const response = await send(() => getSnapshot(fetcher, baseUrl));
        if (response.status === 401) {
          throw syncError('unauthorized', 'unauthorized');
        }
        if (!response.ok) {
          const body = await response.text();
          throw syncError('server', body);
        }
        // response.json() is untyped; the snapshot endpoint's documented payload is a Snapshot
        const snapshotDocument = (await response.json()) as Snapshot;
        await replica.bootstrap(
          { projects: snapshotDocument.projects },
          snapshotDocument.last_seen_seq,
          { onlyIfNeeded: true },
        );
      }

      const current = replica.getSnapshot();
      const response = await send(() =>
        postSync(fetcher, baseUrl, {
          op_version: OPERATION_VERSION,
          last_seen_seq: current.lastSeenSeq,
          ops: [...current.outbox],
        }),
      );

      if (response.status === 401) {
        throw syncError('unauthorized', 'unauthorized');
      }
      if (response.status === 400) {
        const body = await response.text();
        if (body.includes('unsupported operation version')) {
          throw syncError('unsupported_version', body);
        }
        throw syncError('server', body);
      }
      if (!response.ok) {
        const body = await response.text();
        throw syncError('server', body);
      }

      let syncResponse: SyncResponse;
      try {
        // response.json() is untyped; the sync endpoint's documented payload is a SyncResponse
        syncResponse = (await response.json()) as SyncResponse;
      } catch (cause) {
        throw syncError('server', 'invalid sync response', cause);
      }

      await replica.applySyncResult(syncResponse.changes, syncResponse.next_seq);

      const acknowledged: string[] = [];
      const nextRejected: RejectedOperation[] = [];
      let unsupportedVersion = false;

      for (const ack of syncResponse.acks) {
        if (isRejected(ack)) {
          nextRejected.push({ operation_id: ack.operation_id, reason: ack.reason });
          if (ack.reason === 'unsupported_version') {
            unsupportedVersion = true;
          } else {
            acknowledged.push(ack.operation_id);
          }
        } else {
          acknowledged.push(ack.operation_id);
        }
      }

      mergeRejected(nextRejected);
      if (acknowledged.length > 0) {
        await replica.acknowledge(acknowledged);
      }

      if (unsupportedVersion) {
        status = 'error';
        error = syncError('unsupported_version', 'unsupported operation version');
        notify();
        throw error;
      }

      lastSyncedAt = new Date().toISOString();
      persist();
      status = 'idle';
      error = null;
      notify();
    } catch (caught) {
      let mapped: SyncError;
      if (isSyncError(caught)) {
        mapped = caught;
      } else if (isReplicaError(caught)) {
        mapped = syncError('storage', caught.message, caught);
      } else {
        mapped = syncError(
          'server',
          caught instanceof Error ? caught.message : 'sync failed',
          caught,
        );
      }
      status = 'error';
      error = mapped;
      notify();
      throw mapped;
    }
  }

  return {
    getSnapshot() {
      return snapshot;
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    sync() {
      if (inFlight !== null) {
        return inFlight;
      }
      const pending = syncInternal().finally(() => {
        inFlight = null;
      });
      inFlight = pending;
      return pending;
    },
    clearRejectedOperation(operationId) {
      rejected = rejected.filter((item) => item.operation_id !== operationId);
      persist();
      notify();
    },
    clearAllRejectedOperations() {
      rejected = [];
      persist();
      notify();
    },
  };
}

function isRejected(
  ack: OperationAcknowledgment,
): ack is Extract<OperationAcknowledgment, { status: 'rejected' }> {
  return ack.status === 'rejected';
}
