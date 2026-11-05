import type { ClientState } from '@/contract/op-replay';
import { applyChanges as applyConfirmedChanges, replayOperations } from '@/contract/op-replay';
import type { Change } from '@/generated/Change';
import type { Operation } from '@/generated/Operation';
import { requestPersistence } from './persistence';
import {
  deleteOutboxOperations,
  enqueueOperation,
  loadConfirmed,
  loadOutbox,
  openReplicaDatabase,
  saveConfirmed,
  saveLastSeenSeq,
} from './storage';
import { REPLICA_DATABASE } from './types';
import type { ReplicaOptions, ReplicaSnapshot } from './types';

export type Replica = {
  readonly confirmed: ClientState;
  readonly visible: ClientState;
  readonly outbox: readonly Operation[];
  readonly needsBootstrap: boolean;
  readonly persisted: boolean;
  readonly lastSeenSeq: number;
  getSnapshot(): ReplicaSnapshot;
  subscribe(listener: () => void): () => void;
  bootstrap(state: ClientState, lastSeenSeq?: number): Promise<void>;
  applyChanges(changes: readonly Change[]): Promise<void>;
  enqueue(operation: Operation): Promise<void>;
  acknowledge(operationIds: readonly string[]): Promise<void>;
  setLastSeenSeq(lastSeenSeq: number): Promise<void>;
  close(): void;
};

export async function openReplica(options: ReplicaOptions = {}): Promise<Replica> {
  const db = await openReplicaDatabase(options.databaseName ?? REPLICA_DATABASE, options.onBlocked);

  try {
    const loaded = await loadConfirmed(db);
    const persisted = await requestPersistence();
    let state = loaded.state;
    let needsBootstrap = loaded.needsBootstrap;
    let lastSeenSeq = loaded.lastSeenSeq;
    let outbox = await loadOutbox(db);
    let snapshot = snapshotOf(state, outbox, needsBootstrap, persisted, lastSeenSeq);
    const listeners = new Set<() => void>();

    function notify(): void {
      snapshot = snapshotOf(state, outbox, needsBootstrap, persisted, lastSeenSeq);
      for (const listener of [...listeners]) {
        listener();
      }
    }

    return {
      get confirmed() {
        return state;
      },
      get visible() {
        return snapshot.visible;
      },
      get outbox() {
        return snapshot.outbox;
      },
      get needsBootstrap() {
        return needsBootstrap;
      },
      get persisted() {
        return persisted;
      },
      get lastSeenSeq() {
        return lastSeenSeq;
      },
      getSnapshot() {
        return snapshot;
      },
      subscribe(listener) {
        listeners.add(listener);
        return () => {
          listeners.delete(listener);
        };
      },
      async bootstrap(next, lastSeenSeqValue = 0) {
        const clone = structuredClone(next);
        await saveConfirmed(db, clone, lastSeenSeqValue);
        state = clone;
        lastSeenSeq = lastSeenSeqValue;
        needsBootstrap = false;
        notify();
      },
      async applyChanges(changes) {
        const next = applyConfirmedChanges(state, changes);
        await saveConfirmed(db, next);
        state = next;
        notify();
      },
      async enqueue(operation) {
        await enqueueOperation(db, operation);
        outbox = [...outbox, structuredClone(operation)];
        notify();
      },
      async acknowledge(operationIds) {
        await deleteOutboxOperations(db, operationIds);
        const acknowledged = new Set(operationIds);
        outbox = outbox.filter((operation) => !acknowledged.has(operation.operation_id));
        notify();
      },
      async setLastSeenSeq(next) {
        await saveLastSeenSeq(db, next);
        lastSeenSeq = next;
        notify();
      },
      close() {
        db.close();
      },
    };
  } catch (error) {
    db.close();
    throw error;
  }
}

function snapshotOf(
  state: ClientState,
  outbox: readonly Operation[],
  needsBootstrap: boolean,
  persisted: boolean,
  lastSeenSeq: number,
): ReplicaSnapshot {
  return {
    confirmed: state,
    visible: replayOperations(state, outbox),
    outbox: [...outbox],
    needsBootstrap,
    persisted,
    lastSeenSeq,
  };
}
