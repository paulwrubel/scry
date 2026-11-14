import { useSyncExternalStore } from 'react';
import type { Replica, ReplicaSnapshot } from '@/utils/replica';
import { useReplicaContext } from './context';
import type { ReplicaContextValue } from './context';

export function useReplicaStatus(): ReplicaContextValue {
  return useReplicaContext();
}

export function useReplica(): Replica {
  const { status, replica } = useReplicaContext();
  if (status !== 'ready' || replica === undefined) {
    throw new Error('the Replica is not ready');
  }
  return replica;
}

export function useReplicaSnapshot(): ReplicaSnapshot {
  const replica = useReplica();
  return useSyncExternalStore(replica.subscribe, replica.getSnapshot);
}
