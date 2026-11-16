import { DEVICE_ZONE, deadline } from '@/contract/due';
import type { Priority } from '@/generated/Priority';
import type { Task } from '@/generated/Task';
import type { TaskSortingMode } from '@/generated/TaskSortingMode';

const PRIORITY_RANK: Record<Priority, number> = {
  critical: 0,
  elevated: 1,
  normal: 2,
};

export function compareTasks(
  mode: TaskSortingMode,
  left: Task,
  right: Task,
  zone: string = DEVICE_ZONE,
): number {
  const ordering = compareByMode(mode, left, right, zone);
  return ordering === 0 ? compareCodeUnits(left.id, right.id) : ordering;
}

function compareByMode(mode: TaskSortingMode, left: Task, right: Task, zone: string): number {
  switch (mode) {
    case 'manual':
      return compareCodeUnits(left.position, right.position);
    case 'priority': {
      const rank = PRIORITY_RANK[left.priority] - PRIORITY_RANK[right.priority];
      return rank === 0 ? compareCodeUnits(left.position, right.position) : rank;
    }
    case 'due': {
      if (left.due !== null && right.due !== null) {
        const order = deadline(left.due, zone).diff(
          deadline(right.due, zone),
          'milliseconds',
        ).milliseconds;
        return order === 0 ? compareCodeUnits(left.position, right.position) : order;
      }
      if (left.due === null && right.due === null) {
        return compareCodeUnits(left.position, right.position);
      }
      return left.due === null ? 1 : -1;
    }
    default:
      throw new Error(`unknown sorting mode: ${mode}`);
  }
}

function compareCodeUnits(left: string, right: string): number {
  if (left < right) {
    return -1;
  }
  if (left > right) {
    return 1;
  }
  return 0;
}
