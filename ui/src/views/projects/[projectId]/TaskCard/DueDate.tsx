import { formatDue, isOverdue } from '@/contract/due';
import type { Task } from '@/generated/Task';

export type DueDateProps = {
  due: Task['due'];
  state: Task['state'];
};

export function DueDate(props: DueDateProps) {
  const { due, state } = props;

  if (due === null) {
    return null;
  }

  const dueText = formatDue(due);
  const dueLabel = `Due ${dueText}`;
  const overdue = isOverdue({ due, state });
  const label = overdue ? `Overdue, was due ${dueText}` : dueLabel;

  return (
    <p
      className={`ml-auto text-right text-xs ${overdue ? 'text-danger' : 'text-muted'}`}
      aria-label={label}
      title={label}
    >
      {dueLabel}
    </p>
  );
}
