import { useSyncExternalStore } from 'react';
import type { SyncSnapshot } from '@/utils/sync';
import { useSync } from './context';

export function useSyncSnapshot(): SyncSnapshot {
  const { client } = useSync();
  return useSyncExternalStore(client.subscribe, client.getSnapshot);
}
