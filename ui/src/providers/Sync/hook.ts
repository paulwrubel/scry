import { useContext, useSyncExternalStore } from 'react';
import type { SyncSnapshot } from '@/utils/sync';
import { SyncContext } from './context';
import type { SyncContextValue } from './context';

export function useSync(): SyncContextValue {
  const context = useContext(SyncContext);
  if (context === null) {
    throw new Error('useSync must be used within SyncProvider');
  }
  return context;
}

export function useSyncSnapshot(): SyncSnapshot {
  const { client } = useSync();
  return useSyncExternalStore(client.subscribe, client.getSnapshot);
}
