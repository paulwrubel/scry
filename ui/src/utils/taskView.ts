import type { TaskSortingMode } from '@/generated/TaskSortingMode';

export const TASK_SORTING_PARAM = 'sort';

export const DEFAULT_TASK_SORTING_MODE: TaskSortingMode = 'manual';

export const TASK_SORTING_MODES: readonly TaskSortingMode[] = ['manual', 'priority'];

export const TASK_SORTING_OPTIONS: readonly { value: TaskSortingMode; label: string }[] = [
  { value: 'manual', label: 'Manual' },
  { value: 'priority', label: 'Priority' },
];

export function isTaskSortingMode(value: string): value is TaskSortingMode {
  return (TASK_SORTING_MODES as readonly string[]).includes(value);
}

export function parseTaskSortingMode(params: URLSearchParams): TaskSortingMode {
  const raw = params.get(TASK_SORTING_PARAM) ?? '';
  return isTaskSortingMode(raw) ? raw : DEFAULT_TASK_SORTING_MODE;
}

export function applyTaskSortingMode(
  params: URLSearchParams,
  mode: TaskSortingMode,
): URLSearchParams {
  const next = new URLSearchParams(params);
  next.delete(TASK_SORTING_PARAM);
  // the manual default stays out of the URL so a default view keeps a clean link
  if (mode !== DEFAULT_TASK_SORTING_MODE) {
    next.set(TASK_SORTING_PARAM, mode);
  }
  return next;
}
