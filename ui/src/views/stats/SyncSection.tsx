import {
  Alert,
  Badge,
  Button,
  Card,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeadCell,
  TableRow,
} from 'flowbite-react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '@/providers/Auth';
import { useReplicaSnapshot } from '@/providers/Replica';
import { syncStatusColor, useSync, useSyncSnapshot, useSyncStatus } from '@/providers/Sync';
import { describeRejectionReason } from '@/utils/sync';

const tableTheme = { root: { base: 'w-full text-left text-sm text-zinc-300' } };

export function SyncSection() {
  const navigate = useNavigate();
  const { isAuthenticated, logout } = useAuth();
  const replica = useReplicaSnapshot();
  const snapshot = useSyncSnapshot();
  const status = useSyncStatus();
  const { syncNow, clearRejectedOperation, clearAllRejectedOperations } = useSync();

  const syncing = snapshot.status === 'syncing';

  async function handleSignOut() {
    await logout();
    navigate('/login', { replace: true });
  }

  return (
    <div className="flex flex-col gap-6">
      <Card>
        <div className="flex items-center justify-between gap-3">
          <div className="flex flex-col gap-1">
            <h2 className="text-lg font-semibold">Sync</h2>
            <p className="text-muted text-sm">{status.detail}</p>
          </div>
          <Badge color={syncStatusColor(status.status)}>{status.label}</Badge>
        </div>

        <div className="grid grid-cols-2 gap-2 text-sm">
          <span className="text-muted">Pending operations</span>
          <span>{status.pending}</span>
          <span className="text-muted">Last successful sync</span>
          <span>{status.lastSyncedLabel}</span>
          <span className="text-muted">Storage persisted</span>
          <span>{replica.persisted ? 'Yes' : 'No'}</span>
          <span className="text-muted">Account</span>
          <span>{isAuthenticated ? 'Signed in' : 'Signed out'}</span>
        </div>

        {status.status === 'error' ? (
          <Alert color="failure">
            <span className="font-medium">Last sync failed</span>
            <span className="mt-1 block text-sm">{status.detail}</span>
          </Alert>
        ) : null}

        {!replica.persisted ? (
          <Alert color="warning">
            <span className="font-medium">Local storage is not persistent</span>
            <span className="mt-1 block text-sm">
              The browser may evict offline changes. Your work still syncs while online.
            </span>
          </Alert>
        ) : null}

        <div className="flex flex-wrap items-center gap-2">
          <Button
            size="sm"
            color="default"
            disabled={syncing || !isAuthenticated}
            onClick={() => void syncNow()}
          >
            {syncing ? 'Syncing...' : 'Sync now'}
          </Button>
          <Button size="sm" color="alternative" onClick={() => void handleSignOut()}>
            Sign out
          </Button>
        </div>
      </Card>

      {snapshot.rejected.length > 0 ? (
        <section>
          <div className="mb-2 flex items-center justify-between gap-3">
            <h2 className="text-lg font-semibold text-zinc-300">Rejected operations</h2>
            <Button size="xs" color="alternative" onClick={() => clearAllRejectedOperations()}>
              Clear all
            </Button>
          </div>
          <div className="overflow-x-auto rounded-lg border border-zinc-700">
            <Table theme={tableTheme}>
              <TableHead>
                <TableRow>
                  <TableHeadCell>Operation</TableHeadCell>
                  <TableHeadCell>Reason</TableHeadCell>
                  <TableHeadCell>Actions</TableHeadCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {snapshot.rejected.map((rejection) => (
                  <TableRow key={rejection.operation_id}>
                    <TableCell className="font-mono text-xs">
                      {rejection.operation_id.slice(0, 8)}
                    </TableCell>
                    <TableCell className="text-muted">
                      {describeRejectionReason(rejection.reason)}
                    </TableCell>
                    <TableCell>
                      <Button
                        size="xs"
                        color="default"
                        onClick={() => clearRejectedOperation(rejection.operation_id)}
                      >
                        Dismiss
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </section>
      ) : null}
    </div>
  );
}
