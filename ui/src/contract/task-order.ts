import type { Priority } from '../generated/Priority';
import type { Task } from '../generated/Task';
import type { TaskSortingMode } from '../generated/TaskSortingMode';

const PRIORITY_RANK: Record<Priority, number> = {
  critical: 0,
  high: 1,
  medium: 2,
  low: 3,
  minimal: 4,
};

export function compareTasks(mode: TaskSortingMode, left: Task, right: Task): number {
  const ordering = compareByMode(mode, left, right);
  return ordering === 0 ? compareCodeUnits(left.id, right.id) : ordering;
}

function compareByMode(mode: TaskSortingMode, left: Task, right: Task): number {
  switch (mode) {
    case 'alphabetical':
      return compareCodePoints(left.title, right.title);
    case 'alphabetical-case-insensitive':
      return compareCodePoints(left.title.toLowerCase(), right.title.toLowerCase());
    case 'created':
      return compareInstants(left.created_at, right.created_at);
    case 'manual':
      return compareCodeUnits(left.position, right.position);
    case 'priority':
      return PRIORITY_RANK[left.priority] - PRIORITY_RANK[right.priority];
    default:
      throw new Error(`unknown sorting mode: ${mode}`);
  }
}

// Rust compares UTF-8 bytes, which is code point order; `<` compares UTF-16
// code units and disagrees for astral characters.
function compareCodePoints(left: string, right: string): number {
  let leftIndex = 0;
  let rightIndex = 0;

  while (leftIndex < left.length && rightIndex < right.length) {
    const leftPoint = left.codePointAt(leftIndex) ?? 0;
    const rightPoint = right.codePointAt(rightIndex) ?? 0;
    if (leftPoint !== rightPoint) {
      return leftPoint - rightPoint;
    }
    leftIndex += leftPoint > 0xffff ? 2 : 1;
    rightIndex += rightPoint > 0xffff ? 2 : 1;
  }

  return left.length - leftIndex - (right.length - rightIndex);
}

// Date.parse drops sub-millisecond precision, so compare the full RFC 3339
// fraction as text.
function compareInstants(left: string, right: string): number {
  return compareCodeUnits(instantKey(left), instantKey(right));
}

function instantKey(value: string): string {
  const match = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.(\d+))?Z$/.exec(value);
  if (match === null) {
    throw new Error(`expected a UTC timestamp, received ${value}`);
  }
  return `${match[1]}.${(match[2] ?? '').padEnd(9, '0')}`;
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
