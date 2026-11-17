import type { ClosedReason } from '@/generated/ClosedReason';
import type { TaskState } from '@/generated/TaskState';

export const TASK_STATE_LABELS: Record<TaskState, string> = {
  open: 'Open',
  closed: 'Closed',
};

export const CLOSED_REASON_LABELS: Record<ClosedReason, string> = {
  completed: 'Completed',
  cancelled: 'Cancelled',
  skipped: 'Skipped',
};
