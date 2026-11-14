import type { TaskState } from '@/generated/TaskState';

export const TASK_STATE_LABELS: Record<TaskState, string> = {
  open: 'Open',
  closed: 'Closed',
};
