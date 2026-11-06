import { Badge, Tooltip } from 'flowbite-react';
import { useOnlineStatus } from '@/hooks/useOnlineStatus';

export function ConnectionStatus() {
  const online = useOnlineStatus();

  return (
    <Tooltip
      content={
        online
          ? 'Connected to the server.'
          : 'Offline. Changes are saved locally and will sync when the connection returns.'
      }
    >
      <Badge
        color={online ? 'success' : 'warning'}
        size="sm"
        className="cursor-default select-none"
      >
        {online ? 'Online' : 'Offline'}
      </Badge>
    </Tooltip>
  );
}
