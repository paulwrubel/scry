import { createRequiredContext } from '@/hooks/createRequiredContext';
import type { Replica } from '@/utils/replica';

export type ReplicaStatus = 'loading' | 'ready' | 'error';

export type ReplicaContextValue = {
  status: ReplicaStatus;
  replica: Replica | undefined;
  error: Error | undefined;
  retry: () => void;
};

export const [ReplicaContext, useReplicaContext] =
  createRequiredContext<ReplicaContextValue>('Replica');
