import { createContext } from 'react';
import type { SyncClient } from '@/utils/sync';

export type SyncContextValue = {
  client: SyncClient;
  syncNow: () => Promise<void>;
};

export const SyncContext = createContext<SyncContextValue | null>(null);
