import { Badge, Tooltip } from 'flowbite-react';
import { useSyncStatus } from '@/providers/Sync';
import type { SyncDisplayStatus } from '@/providers/Sync';

const COLORS: Record<SyncDisplayStatus, 'success' | 'info' | 'warning' | 'failure' | 'gray'> = {
  checking: 'gray',
  unauthenticated: 'gray',
  offline: 'gray',
  syncing: 'info',
  error: 'failure',
  pending: 'warning',
  synced: 'success',
};

export function SyncStatus() {
  const status = useSyncStatus();
  const details = [
    status.detail,
    status.pending > 0 ? `${status.pending} pending` : null,
    status.rejected > 0 ? `${status.rejected} rejected` : null,
    `Last sync: ${status.lastSyncedLabel}`,
  ]
    .filter((part): part is string => part !== null)
    .join(' · ');

  return (
    <Tooltip content={details}>
      <Badge color={COLORS[status.status]} size="sm" className="cursor-default select-none">
        {status.label}
      </Badge>
    </Tooltip>
  );
}
