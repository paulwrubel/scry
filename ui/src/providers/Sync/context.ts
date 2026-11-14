import { createRequiredContext } from '@/hooks/createRequiredContext';
import type { SyncClient } from '@/utils/sync';

export type SyncContextValue = {
  client: SyncClient;
  syncNow: () => Promise<void>;
  clearRejectedOperation: (operationId: string) => void;
  clearAllRejectedOperations: () => void;
};

export const [SyncContext, useSync] = createRequiredContext<SyncContextValue>('Sync');
