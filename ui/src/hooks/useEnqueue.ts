import { useCallback } from 'react';
import type { Operation } from '@/generated/Operation';
import { useReplica } from '@/providers/Replica';

export type Enqueue = (operation: Operation) => Promise<void>;

/** Stable wrapper around the replica's outbox, for components that build their own operations. */
export function useEnqueue(): Enqueue {
  const replica = useReplica();

  return useCallback((operation) => replica.enqueue(operation), [replica]);
}
