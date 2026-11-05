import { Badge, Card } from 'flowbite-react';
import { useReplicaSnapshot } from '@/providers/Replica';

export function SettingsPage() {
  const snapshot = useReplicaSnapshot();

  return (
    <div className="p-6">
      <Card className="max-w-xl">
        <h1 className="text-2xl font-semibold">Settings</h1>
        <div className="grid grid-cols-2 gap-2 text-sm">
          <span className="text-muted">Local replica</span>
          <span>
            <Badge color={snapshot.needsBootstrap ? 'warning' : 'success'}>
              {snapshot.needsBootstrap ? 'Needs bootstrap' : 'Ready'}
            </Badge>
          </span>
          <span className="text-muted">Storage persisted</span>
          <span>
            <Badge color={snapshot.persisted ? 'success' : 'gray'}>
              {snapshot.persisted ? 'Yes' : 'No'}
            </Badge>
          </span>
          <span className="text-muted">Pending operations</span>
          <span>{snapshot.outbox.length}</span>
        </div>
      </Card>
    </div>
  );
}
