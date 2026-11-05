import { useMemo, useState } from 'react';
import { useSyncTriggers } from '@/hooks/useSyncTriggers';
import type { SyncTiming } from '@/hooks/useSyncTriggers';
import { useReplica, useReplicaSnapshot, useReplicaStatus } from '@/providers/Replica';
import { createSyncClient } from '@/utils/sync';
import type { SyncClient } from '@/utils/sync';
import { SyncContext } from './context';
import type { SyncContextValue } from './context';

export type SyncProviderProps = {
  children: React.ReactNode;
  client?: SyncClient;
  timing?: Partial<SyncTiming>;
};

export function SyncProvider(props: SyncProviderProps) {
  const { status } = useReplicaStatus();
  if (status !== 'ready') {
    return props.children;
  }
  return <ReadySyncProvider {...props} />;
}

function ReadySyncProvider(props: SyncProviderProps) {
  const { children, client: providedClient, timing } = props;

  const replica = useReplica();
  const [client] = useState<SyncClient>(() => providedClient ?? createSyncClient({ replica }));
  const snapshot = useReplicaSnapshot();
  const syncNow = useSyncTriggers({
    client,
    outboxLength: snapshot.outbox.length,
    timing,
  });

  const value = useMemo<SyncContextValue>(() => ({ client, syncNow }), [client, syncNow]);

  return <SyncContext.Provider value={value}>{children}</SyncContext.Provider>;
}
