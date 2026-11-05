import { Button } from 'flowbite-react';
import { useSync, useSyncSnapshot } from '@/providers/Sync';

export function SyncButton() {
  const { isLeader, syncNow } = useSync();
  const snapshot = useSyncSnapshot();
  const syncing = snapshot.status === 'syncing';
  const failed = snapshot.status === 'error';

  function handleClick() {
    void syncNow();
  }

  return (
    <Button
      size="xs"
      color={failed ? 'failure' : 'alternative'}
      disabled={!isLeader || syncing}
      title={!isLeader ? 'Sync is handled by another tab' : (snapshot.error?.message ?? 'Sync now')}
      onClick={handleClick}
    >
      {syncing ? 'Syncing...' : 'Sync'}
    </Button>
  );
}
