import { useEffect, useState } from 'react';
import { useOnlineStatus } from '@/hooks/useOnlineStatus';
import { useAuth } from '@/providers/Auth';
import { useReplicaSnapshot } from '@/providers/Replica';
import type { SyncError, SyncSnapshot } from '@/utils/sync';
import { useSyncSnapshot } from './hook';

export type SyncDisplayStatus =
  'checking' | 'unauthenticated' | 'offline' | 'syncing' | 'error' | 'pending' | 'synced';

export type SyncStatusView = {
  status: SyncDisplayStatus;
  label: string;
  detail: string;
  pending: number;
  lastSyncedAt: string | null;
  lastSyncedLabel: string;
  rejected: number;
  persisted: boolean;
};

export type DeriveSyncStatusInput = {
  snapshot: SyncSnapshot;
  isAuthenticated: boolean;
  isAuthLoading: boolean;
  online: boolean;
  pending: number;
  syncingVisible: boolean;
  persisted: boolean;
};

export function syncStatusColor(
  status: SyncDisplayStatus,
): 'success' | 'info' | 'warning' | 'failure' | 'gray' {
  switch (status) {
    case 'syncing':
      return 'info';
    case 'pending':
      return 'warning';
    case 'error':
      return 'failure';
    case 'synced':
      return 'success';
    default:
      return 'gray';
  }
}

export function formatLastSyncedAt(value: string | null): string {
  if (value === null) {
    return 'Never';
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return 'Never';
  }
  return date.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
}

export function deriveSyncStatus(input: DeriveSyncStatusInput): SyncStatusView {
  const { snapshot, isAuthenticated, isAuthLoading, online, pending, syncingVisible, persisted } =
    input;
  const lastSyncedLabel = formatLastSyncedAt(snapshot.lastSyncedAt);
  const rejected = snapshot.rejected.length;
  const base = {
    pending,
    lastSyncedAt: snapshot.lastSyncedAt,
    lastSyncedLabel,
    rejected,
    persisted,
  };

  if (isAuthLoading) {
    return { ...base, status: 'checking', label: 'Checking', detail: 'Checking your session.' };
  }
  if (!isAuthenticated) {
    return {
      ...base,
      status: 'unauthenticated',
      label: 'Signed out',
      detail: 'Sign in to sync changes.',
    };
  }
  if (!online) {
    return {
      ...base,
      status: 'offline',
      label: 'Offline',
      detail: 'Changes are saved locally and will sync when you reconnect.',
    };
  }
  if (syncingVisible) {
    return {
      ...base,
      status: 'syncing',
      label: 'Syncing',
      detail: pending > 0 ? `Syncing ${count(pending, 'pending change')}.` : 'Syncing changes.',
    };
  }
  if (snapshot.status === 'error') {
    if (snapshot.error?.kind === 'unauthorized') {
      return {
        ...base,
        status: 'unauthenticated',
        label: 'Signed out',
        detail: 'Sign in to resume sync.',
      };
    }
    return { ...base, status: 'error', label: 'Sync error', detail: describeError(snapshot.error) };
  }
  if (pending > 0) {
    return {
      ...base,
      status: 'pending',
      label: `${pending} pending`,
      detail: `${count(pending, 'change')} waiting to sync.`,
    };
  }
  if (rejected > 0) {
    return {
      ...base,
      status: 'error',
      label: 'Sync issue',
      detail: `${count(rejected, 'operation')} rejected. Open settings for details.`,
    };
  }
  return {
    ...base,
    status: 'synced',
    label: 'Synced',
    detail:
      lastSyncedLabel === 'Never'
        ? 'Waiting for the first sync.'
        : `Last synced ${lastSyncedLabel}.`,
  };
}

export function useSyncStatus(): SyncStatusView {
  const snapshot = useSyncSnapshot();
  const replica = useReplicaSnapshot();
  const { isAuthenticated, isAuthLoading } = useAuth();
  const online = useOnlineStatus();
  const syncingVisible = useSyncingVisible(snapshot.status === 'syncing');

  return deriveSyncStatus({
    snapshot,
    isAuthenticated,
    isAuthLoading,
    online,
    pending: replica.outbox.length,
    syncingVisible,
    persisted: replica.persisted,
  });
}

function useSyncingVisible(active: boolean, delayMs = 300): boolean {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const timer = window.setTimeout(() => setVisible(active), active ? delayMs : 0);
    return () => window.clearTimeout(timer);
  }, [active, delayMs]);

  return visible;
}

function describeError(error: SyncError | null): string {
  switch (error?.kind) {
    case 'unsupported_version':
      return 'This app version is not supported by the server. Update Scry to sync.';
    case 'network':
      return 'Cannot reach the server. Changes stay local until the connection returns.';
    case 'storage':
      return 'Local storage failed. Changes may not be saved.';
    case 'unauthorized':
      return 'Sign in to resume sync.';
    case 'server':
      return error.message.length > 0 ? error.message : 'The last sync failed.';
    default:
      return 'The last sync failed.';
  }
}

function count(value: number, noun: string): string {
  return `${value} ${noun}${value === 1 ? '' : 's'}`;
}
