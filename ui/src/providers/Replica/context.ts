import { createContext } from 'react';
import type { Replica } from '@/utils/replica';

export type ReplicaStatus = 'loading' | 'ready' | 'error';

export type ReplicaContextValue = {
  status: ReplicaStatus;
  replica: Replica | undefined;
  error: Error | undefined;
  retry: () => void;
};

export const ReplicaContext = createContext<ReplicaContextValue | null>(null);
