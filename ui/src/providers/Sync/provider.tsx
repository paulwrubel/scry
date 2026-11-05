import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import { useSyncLeadership } from '@/hooks/useSyncLeadership';
import type { SyncChannelMessage } from '@/hooks/useSyncLeadership';
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
  lockManager?: LockManager | null;
  channel?: BroadcastChannel | null;
};

export function SyncProvider(props: SyncProviderProps) {
  const { status } = useReplicaStatus();
  if (status !== 'ready') {
    return props.children;
  }
  return <ReadySyncProvider {...props} />;
}

function ReadySyncProvider(props: SyncProviderProps) {
  const { children, client: providedClient, timing, lockManager, channel } = props;

  const { isAuthenticated } = useAuth();
  const replica = useReplica();
  const [client] = useState<SyncClient>(() => providedClient ?? createSyncClient({ replica }));
  const clientSnapshot = useSyncExternalStore(client.subscribe, client.getSnapshot);
  const replicaSnapshot = useReplicaSnapshot();

  const isLeaderRef = useRef(false);
  const isAuthenticatedRef = useRef(false);
  useEffect(() => {
    isAuthenticatedRef.current = isAuthenticated;
  }, [isAuthenticated]);

  const reloadAndSync = useCallback(() => {
    if (!isAuthenticatedRef.current) {
      return;
    }
    void replica
      .reload()
      .then(() => client.sync())
      .catch(() => {
        // the client snapshot and leadership state surface failures
      });
  }, [client, replica]);

  const handleMessage = useCallback(
    (message: SyncChannelMessage) => {
      if (message.type === 'replica-updated') {
        void replica.reload();
        return;
      }
      if (isLeaderRef.current) {
        reloadAndSync();
      }
    },
    [reloadAndSync, replica],
  );

  const { isLeader, notify } = useSyncLeadership({
    onLeader: reloadAndSync,
    onMessage: handleMessage,
    lockManager,
    channel,
  });

  useEffect(() => {
    isLeaderRef.current = isLeader;
  }, [isLeader]);

  const syncNow = useSyncTriggers({
    client,
    outboxLength: replicaSnapshot.outbox.length,
    timing,
    enabled: isLeader && isAuthenticated,
  });

  // drain the outbox queued while signed out as soon as auth returns
  useEffect(() => {
    if (isAuthenticated) {
      void client.sync().catch(() => {
        // the client snapshot surfaces the failure
      });
    }
  }, [client, isAuthenticated]);

  // a follower's edit belongs to the leader, so announce it instead of syncing
  const previousOutboxLength = useRef(replicaSnapshot.outbox.length);
  useEffect(() => {
    if (replicaSnapshot.outbox.length > previousOutboxLength.current && !isLeader) {
      notify({ type: 'outbox-changed' });
    }
    previousOutboxLength.current = replicaSnapshot.outbox.length;
  }, [isLeader, notify, replicaSnapshot.outbox.length]);

  // tell followers when a leader sync finishes so they reload fresh state
  const previousStatus = useRef(clientSnapshot.status);
  useEffect(() => {
    if (isLeader && previousStatus.current === 'syncing' && clientSnapshot.status !== 'syncing') {
      notify({ type: 'replica-updated' });
    }
    previousStatus.current = clientSnapshot.status;
  }, [clientSnapshot.status, isLeader, notify]);

  const value = useMemo<SyncContextValue>(
    () => ({ client, isLeader, syncNow }),
    [client, isLeader, syncNow],
  );

  return <SyncContext.Provider value={value}>{children}</SyncContext.Provider>;
}
