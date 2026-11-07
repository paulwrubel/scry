import type { Task } from '@/generated/Task';
import type { TaskState } from '@/generated/TaskState';

export const TASK_STATES = ['open', 'closed'] as const;

export type TaskFilter = {
  states: TaskState[];
  tags: string[];
  search: string;
};

export type TaskFilterStatus = TaskState | 'all';

export const TASK_FILTER_STATUS_OPTIONS: readonly { value: TaskFilterStatus; label: string }[] = [
  { value: 'open', label: 'Open' },
  { value: 'closed', label: 'Closed' },
  { value: 'all', label: 'All' },
];

export const DEFAULT_TASK_FILTER: TaskFilter = {
  states: ['open', 'closed'],
  tags: [],
  search: '',
};

export const FILTER_STATE_PARAM = 'state';
export const FILTER_TAGS_PARAM = 'tags';
export const FILTER_SEARCH_PARAM = 'q';

export function isTaskState(value: string): value is TaskState {
  return (TASK_STATES as readonly string[]).includes(value);
}

export function canonicalStates(states: readonly TaskState[]): TaskState[] {
  return TASK_STATES.filter((state) => states.includes(state));
}

export function matchesTaskFilter(task: Task, filter: TaskFilter): boolean {
  if (!filter.states.includes(task.state)) {
    return false;
  }

  if (filter.tags.length > 0) {
    const taskTags = task.tags.map((tag) => tag.trim().toLowerCase());
    if (!taskTags.some((tag) => filter.tags.includes(tag))) {
      return false;
    }
  }

  const search = filter.search.trim().toLowerCase();
  if (search.length > 0) {
    const inTitle = task.title.toLowerCase().includes(search);
    const inDescription = (task.description ?? '').toLowerCase().includes(search);
    if (!inTitle && !inDescription) {
      return false;
    }
  }

  return true;
}

export function isTaskFilterNarrowing(filter: TaskFilter): boolean {
  return (
    filter.states.length < TASK_STATES.length ||
    filter.tags.length > 0 ||
    filter.search.trim().length > 0
  );
}

export function isTaskFilterDefault(filter: TaskFilter): boolean {
  return (
    canonicalStates(filter.states).length === TASK_STATES.length &&
    filter.tags.length === 0 &&
    filter.search.trim().length === 0
  );
}

export function activeFilterCount(filter: TaskFilter): number {
  let count = 0;
  if (canonicalStates(filter.states).length < TASK_STATES.length) {
    count += 1;
  }
  count += filter.tags.length;
  if (filter.search.trim().length > 0) {
    count += 1;
  }
  return count;
}

export function collectProjectTags(
  tasks: readonly Task[],
  selected: readonly string[] = [],
): string[] {
  const tags = new Set<string>();
  for (const task of tasks) {
    for (const tag of task.tags) {
      const normalized = tag.trim().toLowerCase();
      if (normalized.length > 0) {
        tags.add(normalized);
      }
    }
  }
  // selected tags survive even when no current task carries them, so a stale
  // selection can still be toggled off in the UI
  for (const tag of selected) {
    const normalized = tag.trim().toLowerCase();
    if (normalized.length > 0) {
      tags.add(normalized);
    }
  }
  return [...tags].sort();
}

export function countTasksByTag(tasks: readonly Task[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const task of tasks) {
    const tags = new Set(
      task.tags.map((tag) => tag.trim().toLowerCase()).filter((tag) => tag.length > 0),
    );
    for (const tag of tags) {
      counts.set(tag, (counts.get(tag) ?? 0) + 1);
    }
  }
  return counts;
}

export function parseTaskFilter(params: URLSearchParams): TaskFilter {
  const rawStates = params.get(FILTER_STATE_PARAM);
  const parsedStates = (rawStates ?? '')
    .split(',')
    .map((state) => state.trim())
    .filter(isTaskState);
  const states =
    parsedStates.length > 0 ? canonicalStates(parsedStates) : [...DEFAULT_TASK_FILTER.states];

  const rawTags = params.get(FILTER_TAGS_PARAM) ?? '';
  const tags = [
    ...new Set(
      rawTags
        .split(',')
        .map((tag) => tag.trim().toLowerCase())
        .filter((tag) => tag.length > 0),
    ),
  ].sort();

  const search = (params.get(FILTER_SEARCH_PARAM) ?? '').trim();

  return { states, tags, search };
}

export function filterToSearchParams(filter: TaskFilter): URLSearchParams {
  const params = new URLSearchParams();
  const states = canonicalStates(filter.states);
  // omitting the param entirely means "all states", so default filters yield an empty URL
  if (states.length < TASK_STATES.length) {
    params.set(FILTER_STATE_PARAM, states.join(','));
  }

  if (filter.tags.length > 0) {
    params.set(FILTER_TAGS_PARAM, [...filter.tags].sort().join(','));
  }

  const search = filter.search.trim();
  if (search.length > 0) {
    params.set(FILTER_SEARCH_PARAM, search);
  }

  return params;
}

export function applyTaskFilter(params: URLSearchParams, filter: TaskFilter): URLSearchParams {
  const next = new URLSearchParams(params);
  next.delete(FILTER_STATE_PARAM);
  next.delete(FILTER_TAGS_PARAM);
  next.delete(FILTER_SEARCH_PARAM);
  for (const [key, value] of filterToSearchParams(filter)) {
    next.set(key, value);
  }
  return next;
}

export function taskFilterStatus(filter: TaskFilter): TaskFilterStatus {
  const states = canonicalStates(filter.states);
  if (states.length === TASK_STATES.length) {
    return 'all';
  }
  return states[0] ?? 'all';
}

export function setTaskFilterStatus(filter: TaskFilter, status: TaskFilterStatus): TaskFilter {
  const states = status === 'all' ? [...TASK_STATES] : [status];
  return { ...filter, states };
}

export function toggleTaskFilterTag(filter: TaskFilter, tag: string): TaskFilter {
  const normalized = tag.trim().toLowerCase();
  if (normalized.length === 0) {
    return filter;
  }

  const tags = filter.tags.includes(normalized)
    ? filter.tags.filter((current) => current !== normalized)
    : [...new Set([...filter.tags, normalized])];

  return { ...filter, tags: [...tags].sort() };
}

export function setTaskFilterSearch(filter: TaskFilter, search: string): TaskFilter {
  return { ...filter, search };
}

export function clearTaskFilter(): TaskFilter {
  return { states: [...DEFAULT_TASK_FILTER.states], tags: [], search: '' };
}
