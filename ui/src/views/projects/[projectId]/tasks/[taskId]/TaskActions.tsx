import { Button, Dropdown, DropdownItem } from 'flowbite-react';
import type { ClosedReason } from '@/generated/ClosedReason';
import type { Task } from '@/generated/Task';

export type TaskActionsProps = {
  task: Task;
  onClose: (reason: ClosedReason) => Promise<void>;
  onReopen: () => Promise<void>;
  onRequestDelete: () => void;
};

export function TaskActions(props: TaskActionsProps) {
  const { task, onClose, onReopen, onRequestDelete } = props;

  const closed = task.state === 'closed';
  const stateLocked = (task.external_metadata?.state ?? null) !== null;

  return (
    <div className="flex flex-wrap items-center gap-2">
      {stateLocked ? null : closed ? (
        <Button size="xs" color="alternative" onClick={() => void onReopen()}>
          Reopen
        </Button>
      ) : (
        <Dropdown size="xs" color="alternative" label="Close as" dismissOnClick>
          <DropdownItem onClick={() => void onClose('completed')}>Completed</DropdownItem>
          <DropdownItem onClick={() => void onClose('cancelled')}>Cancelled</DropdownItem>
          <DropdownItem onClick={() => void onClose('skipped')}>Skipped</DropdownItem>
        </Dropdown>
      )}
      <Button size="xs" color="red" onClick={onRequestDelete}>
        Delete
      </Button>
    </div>
  );
}
