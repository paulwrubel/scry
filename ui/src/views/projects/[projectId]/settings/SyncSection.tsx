import {
  Alert,
  Button,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeadCell,
  TableRow,
} from 'flowbite-react';
import { useReplicaSnapshot } from '@/providers/Replica';
import { useSync, useSyncSnapshot } from '@/providers/Sync';
import { describeRejectionReason } from '@/utils/sync';

const tableTheme = { root: { base: 'w-full text-left text-sm text-zinc-300' } };

export function SyncSection() {
  const replica = useReplicaSnapshot();
  const snapshot = useSyncSnapshot();
  const { clearRejectedOperation, clearAllRejectedOperations } = useSync();

  if (replica.persisted && snapshot.rejected.length === 0) {
    return null;
  }

  return (
    <div className="flex flex-col gap-4">
      {!replica.persisted ? (
        <Alert color="warning">
          <span className="font-medium">Local storage is not persistent</span>
          <span className="mt-1 block text-sm">
            The browser may evict offline changes. Your work still syncs while online.
          </span>
        </Alert>
      ) : null}

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
