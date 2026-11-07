import { Button } from 'flowbite-react';
import { useAuth } from '@/providers/Auth';
import { useSync, useSyncSnapshot, useSyncingVisible } from '@/providers/Sync';

export function SyncButton() {
  const { isAuthenticated } = useAuth();
  const { syncNow } = useSync();
  const snapshot = useSyncSnapshot();
  const syncingNow = snapshot.status === 'syncing';
  const showSyncing = useSyncingVisible(syncingNow);
  const failed = snapshot.status === 'error';

  return (
    <Button
      size="xs"
      color={failed ? 'failure' : 'alternative'}
      disabled={!isAuthenticated || syncingNow}
      title={!isAuthenticated ? 'Sign in to sync' : (snapshot.error?.message ?? 'Sync now')}
      onClick={() => void syncNow()}
      className="min-w-20"
    >
      {showSyncing ? 'Syncing' : 'Sync'}
    </Button>
  );
}
