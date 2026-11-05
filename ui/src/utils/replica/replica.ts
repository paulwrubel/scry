import type { ClientState } from '@/contract/op-replay';
import { loadConfirmed, openReplicaDatabase, saveConfirmed } from './storage';
import { REPLICA_DATABASE } from './types';
import type { ReplicaOptions } from './types';

export type Replica = {
  readonly confirmed: ClientState;
  bootstrap(state: ClientState): Promise<void>;
  close(): void;
};

export async function openReplica(options: ReplicaOptions = {}): Promise<Replica> {
  const db = await openReplicaDatabase(options.databaseName ?? REPLICA_DATABASE, options.onBlocked);

  try {
    let state = await loadConfirmed(db);

    return {
      get confirmed() {
        return state;
      },
      async bootstrap(next) {
        const clone = structuredClone(next);
        await saveConfirmed(db, clone);
        state = clone;
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
