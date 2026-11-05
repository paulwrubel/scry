import { Button } from 'flowbite-react';
import { useAuth } from '@/providers/Auth';
import { useSync, useSyncSnapshot } from '@/providers/Sync';

export function SyncButton() {
  const { isAuthenticated } = useAuth();
  const { syncNow } = useSync();
  const snapshot = useSyncSnapshot();
  const syncing = snapshot.status === 'syncing';
  const failed = snapshot.status === 'error';

  return (
    <Button
      size="xs"
      color={failed ? 'failure' : 'alternative'}
      disabled={!isAuthenticated || syncing}
      title={!isAuthenticated ? 'Sign in to sync' : (snapshot.error?.message ?? 'Sync now')}
      onClick={() => void syncNow()}
    >
      {syncing ? 'Syncing...' : 'Sync'}
    </Button>
  );
}
