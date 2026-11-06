import { Badge, Tooltip } from 'flowbite-react';
import { syncStatusColor, useSyncStatus } from '@/providers/Sync';

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
      <Badge
        color={syncStatusColor(status.status)}
        size="sm"
        className="cursor-default select-none"
      >
        {status.label}
      </Badge>
    </Tooltip>
  );
}
