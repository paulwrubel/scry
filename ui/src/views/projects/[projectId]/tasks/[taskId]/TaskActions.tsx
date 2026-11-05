import { Button } from 'flowbite-react';
import type { Task } from '@/generated/Task';

export type TaskActionsProps = {
  task: Task;
  onToggleState: () => Promise<void>;
  onRequestDelete: () => void;
};

export function TaskActions(props: TaskActionsProps) {
  const { task, onToggleState, onRequestDelete } = props;
  const closed = task.state === 'closed';

  return (
    <div className="flex flex-wrap items-center gap-2">
      <Button size="xs" color="alternative" onClick={() => void onToggleState()}>
        {closed ? 'Reopen task' : 'Close task'}
      </Button>
      <Button size="xs" color="red" onClick={onRequestDelete}>
        Delete
      </Button>
    </div>
  );
}
