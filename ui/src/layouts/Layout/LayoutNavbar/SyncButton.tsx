import { Button } from 'flowbite-react';
import { useSync, useSyncSnapshot } from '@/providers/Sync';

export function SyncButton() {
  const { syncNow } = useSync();
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
      disabled={syncing}
      title={snapshot.error?.message ?? 'Sync now'}
      onClick={handleClick}
    >
      {syncing ? 'Syncing...' : 'Sync'}
    </Button>
  );
}
