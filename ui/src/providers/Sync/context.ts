import { createContext } from 'react';
import type { SyncClient } from '@/utils/sync';

export type SyncContextValue = {
  client: SyncClient;
  isLeader: boolean;
  syncNow: () => Promise<void>;
};

export const SyncContext = createContext<SyncContextValue | null>(null);
