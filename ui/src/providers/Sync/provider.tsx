import { useCallback, useEffect, useMemo, useState } from 'react';
import { useSyncTriggers } from '@/hooks/useSyncTriggers';
import type { SyncTiming } from '@/hooks/useSyncTriggers';
import { useAuth } from '@/providers/Auth';
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

  const { isAuthenticated, authenticatedFetch } = useAuth();
  const replica = useReplica();
  const [client] = useState<SyncClient>(
    () => providedClient ?? createSyncClient({ replica, fetcher: authenticatedFetch }),
  );
  const replicaSnapshot = useReplicaSnapshot();

  const syncNow = useSyncTriggers({
    client,
    outboxLength: replicaSnapshot.outbox.length,
    timing,
    enabled: isAuthenticated,
  });

  // drain the outbox queued while signed out as soon as auth returns
  useEffect(() => {
    if (isAuthenticated) {
      void client.sync().catch(() => {
        // the client snapshot surfaces the failure
      });
    }
  }, [client, isAuthenticated]);

  const clearRejectedOperation = useCallback(
    (operationId: string) => client.clearRejectedOperation(operationId),
    [client],
  );
  const clearAllRejectedOperations = useCallback(
    () => client.clearAllRejectedOperations(),
    [client],
  );

  const value = useMemo<SyncContextValue>(
    () => ({ client, syncNow, clearRejectedOperation, clearAllRejectedOperations }),
    [client, syncNow, clearRejectedOperation, clearAllRejectedOperations],
  );

  return <SyncContext.Provider value={value}>{children}</SyncContext.Provider>;
}
