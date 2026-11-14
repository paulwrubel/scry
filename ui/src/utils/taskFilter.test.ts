import { describe, expect, it } from 'vitest';
import type { Task } from '@/generated/Task';
import {
  DEFAULT_TASK_FILTER,
  TASK_STATES,
  activeFilterCount,
  applyTaskFilter,
  canonicalStates,
  clearTaskFilter,
  collectProjectTags,
  countTasksByTag,
  filterToSearchParams,
  isTaskFilterDefault,
  isTaskFilterNarrowing,
  matchesTaskFilter,
  parseTaskFilter,
  setTaskFilterSearch,
  setTaskFilterStatus,
  taskFilterStatus,
  toggleTaskFilterTag,
  type TaskFilter,
} from './taskFilter';

const CREATED_AT = '2026-01-01T00:00:00Z';

function task(overrides: Partial<Task> = {}): Task {
  return {
    id: 'task-1',
    project_id: 'project-1',
    title: 'Task',
    description: null,
    priority: 'normal',
    section_id: null,
    position: 'a0',
    state: 'open',
    tags: [],
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
    external_identity: null,
    external_metadata: null,
    ...overrides,
  };
}

function makeFilter(overrides: Partial<TaskFilter> = {}): TaskFilter {
  return {
    states: [...TASK_STATES],
    tags: [],
    search: '',
    ...overrides,
  };
}

describe('parseTaskFilter', () => {
  it('returns the default filter for empty params', () => {
    const filter = parseTaskFilter(new URLSearchParams());

    expect(filter.states).toEqual(['open', 'closed']);
    expect(filter.tags).toEqual([]);
    expect(filter.search).toBe('');
  });

  it('parses a single state', () => {
    expect(parseTaskFilter(new URLSearchParams('state=closed')).states).toEqual(['closed']);
  });

  it('parses and orders multiple states', () => {
    expect(parseTaskFilter(new URLSearchParams('state=open,closed')).states).toEqual([
      'open',
      'closed',
    ]);
  });

  it('falls back to all states for unknown or empty state values', () => {
    expect(parseTaskFilter(new URLSearchParams('state=banana')).states).toEqual(['open', 'closed']);
    expect(parseTaskFilter(new URLSearchParams('state=')).states).toEqual(['open', 'closed']);
  });

  it('normalizes, dedupes, and sorts tags', () => {
    const filter = parseTaskFilter(new URLSearchParams('tags=%20Bug%20,URGENT,bug'));

    expect(filter.tags).toEqual(['bug', 'urgent']);
  });

  it('trims the search value', () => {
    expect(parseTaskFilter(new URLSearchParams('q=%20hello%20')).search).toBe('hello');
  });
});

describe('filterToSearchParams', () => {
  it('serializes the default filter to an empty string', () => {
    expect(filterToSearchParams(DEFAULT_TASK_FILTER).toString()).toBe('');
  });

  it('serializes narrowed states, sorted tags, and search', () => {
    const params = filterToSearchParams({
      states: ['closed'],
      tags: ['urgent', 'bug'],
      search: 'fix',
    });

    expect(params.get('state')).toBe('closed');
    expect(params.get('tags')).toBe('bug,urgent');
    expect(params.get('q')).toBe('fix');
  });

  it('omits the state param when all states are selected', () => {
    const params = filterToSearchParams({
      states: ['open', 'closed'],
      tags: ['bug'],
      search: 'fix',
    });

    expect(params.has('state')).toBe(false);
    expect(params.get('tags')).toBe('bug');
    expect(params.get('q')).toBe('fix');
  });
});

describe('applyTaskFilter', () => {
  it('clears filter params while preserving unrelated params', () => {
    const params = new URLSearchParams('redirect=%2Ffoo&state=closed&tags=bug&q=x');

    const next = applyTaskFilter(params, clearTaskFilter());

    expect(next.get('redirect')).toBe('/foo');
    expect(next.has('state')).toBe(false);
    expect(next.has('tags')).toBe(false);
    expect(next.has('q')).toBe(false);
  });
});

describe('matchesTaskFilter', () => {
  it('requires the task state to be selected', () => {
    const closed = task({ state: 'closed' });

    expect(matchesTaskFilter(closed, makeFilter({ states: ['open'] }))).toBe(false);
    expect(matchesTaskFilter(closed, makeFilter({ states: ['open', 'closed'] }))).toBe(true);
  });

  it('matches when the task has any selected tag', () => {
    const tagged = task({ tags: ['bug', 'urgent'] });

    expect(matchesTaskFilter(tagged, makeFilter({ tags: ['bug'] }))).toBe(true);
    expect(matchesTaskFilter(tagged, makeFilter({ tags: ['bug', 'urgent'] }))).toBe(true);
  });

  it('rejects tasks without a selected tag', () => {
    const tagged = task({ tags: ['feature'] });

    expect(matchesTaskFilter(tagged, makeFilter({ tags: ['bug'] }))).toBe(false);
  });

  it('matches any task when no tags are selected', () => {
    expect(matchesTaskFilter(task({ tags: ['feature'] }), makeFilter({ tags: [] }))).toBe(true);
  });

  it('searches titles case-insensitively', () => {
    const matching = task({ title: 'Fix login bug' });

    expect(matchesTaskFilter(matching, makeFilter({ search: 'LOGIN' }))).toBe(true);
  });

  it('searches descriptions case-insensitively', () => {
    const matching = task({ description: 'Auth flow' });

    expect(matchesTaskFilter(matching, makeFilter({ search: 'auth' }))).toBe(true);
  });

  it('rejects tasks without a search match', () => {
    expect(matchesTaskFilter(task({ title: 'Task' }), makeFilter({ search: 'nope' }))).toBe(false);
  });

  it('does not search tags', () => {
    const tagged = task({ title: 'Task', description: null, tags: ['backend'] });

    expect(matchesTaskFilter(tagged, makeFilter({ search: 'backend' }))).toBe(false);
  });
});

describe('isTaskFilterNarrowing', () => {
  it('is false for the default filter', () => {
    expect(isTaskFilterNarrowing(DEFAULT_TASK_FILTER)).toBe(false);
  });

  it('is true when states are narrowed', () => {
    expect(isTaskFilterNarrowing(makeFilter({ states: ['open'] }))).toBe(true);
    expect(isTaskFilterNarrowing(makeFilter({ states: ['closed'] }))).toBe(true);
  });

  it('is true when tags are selected', () => {
    expect(isTaskFilterNarrowing(makeFilter({ tags: ['bug'] }))).toBe(true);
  });

  it('is true when search has content', () => {
    expect(isTaskFilterNarrowing(makeFilter({ search: 'x' }))).toBe(true);
  });

  it('is false when search is only whitespace', () => {
    expect(isTaskFilterNarrowing(makeFilter({ search: '   ' }))).toBe(false);
  });
});

describe('isTaskFilterDefault', () => {
  it('is true for the default filter and equivalent values', () => {
    expect(isTaskFilterDefault(DEFAULT_TASK_FILTER)).toBe(true);
    expect(isTaskFilterDefault({ states: ['open', 'closed'], tags: [], search: '' })).toBe(true);
  });

  it('is false when states are narrowed', () => {
    expect(isTaskFilterDefault(makeFilter({ states: ['open'] }))).toBe(false);
  });

  it('is false when tags are selected', () => {
    expect(isTaskFilterDefault(makeFilter({ tags: ['bug'] }))).toBe(false);
  });

  it('is false when search has content', () => {
    expect(isTaskFilterDefault(makeFilter({ search: 'x' }))).toBe(false);
  });
});

describe('activeFilterCount', () => {
  it('is 0 for the default filter', () => {
    expect(activeFilterCount(DEFAULT_TASK_FILTER)).toBe(0);
  });

  it('counts a narrowed state selection as one', () => {
    expect(activeFilterCount(makeFilter({ states: ['open'] }))).toBe(1);
  });

  it('counts each selected tag', () => {
    expect(activeFilterCount(makeFilter({ tags: ['bug', 'urgent'] }))).toBe(2);
  });

  it('counts a search as one', () => {
    expect(activeFilterCount(makeFilter({ search: 'x' }))).toBe(1);
  });

  it('sums states, tags, and search', () => {
    expect(activeFilterCount({ states: ['open'], tags: ['bug'], search: 'x' })).toBe(3);
  });
});

describe('canonicalStates', () => {
  it('dedupes and orders states as open then closed', () => {
    expect(canonicalStates(['closed', 'open', 'open'])).toEqual(['open', 'closed']);
    expect(canonicalStates(['closed'])).toEqual(['closed']);
  });

  it('preserves the canonical order of all states', () => {
    expect(canonicalStates([...TASK_STATES].reverse())).toEqual(['open', 'closed']);
  });
});

describe('taskFilterStatus', () => {
  it('reports all when both states are selected', () => {
    expect(taskFilterStatus(DEFAULT_TASK_FILTER)).toBe('all');
  });

  it('reports the single selected state', () => {
    expect(taskFilterStatus(makeFilter({ states: ['open'] }))).toBe('open');
    expect(taskFilterStatus(makeFilter({ states: ['closed'] }))).toBe('closed');
  });

  it('falls back to all for an empty selection', () => {
    expect(taskFilterStatus(makeFilter({ states: [] }))).toBe('all');
  });
});

describe('setTaskFilterStatus', () => {
  it('selects a single state', () => {
    expect(setTaskFilterStatus(DEFAULT_TASK_FILTER, 'open').states).toEqual(['open']);
    expect(setTaskFilterStatus(DEFAULT_TASK_FILTER, 'closed').states).toEqual(['closed']);
  });

  it('selects both states for all', () => {
    expect(setTaskFilterStatus(makeFilter({ states: ['open'] }), 'all').states).toEqual([
      'open',
      'closed',
    ]);
  });

  it('does not mutate the original filter', () => {
    const filter = DEFAULT_TASK_FILTER;

    setTaskFilterStatus(filter, 'open');

    expect(filter.states).toEqual(['open', 'closed']);
  });
});

describe('toggleTaskFilterTag', () => {
  it('normalizes and adds a tag', () => {
    expect(toggleTaskFilterTag(DEFAULT_TASK_FILTER, ' Bug ').tags).toEqual(['bug']);
  });

  it('removes an existing tag', () => {
    const withBug = toggleTaskFilterTag(DEFAULT_TASK_FILTER, ' Bug ');

    expect(toggleTaskFilterTag(withBug, 'bug').tags).toEqual([]);
  });

  it('does nothing for an empty tag', () => {
    const filter = makeFilter();

    expect(toggleTaskFilterTag(filter, '')).toBe(filter);
  });

  it('keeps tags sorted', () => {
    const withUrgent = makeFilter({ tags: ['urgent'] });

    expect(toggleTaskFilterTag(withUrgent, 'bug').tags).toEqual(['bug', 'urgent']);
  });
});

describe('setTaskFilterSearch', () => {
  it('returns a new filter with the raw search value', () => {
    const filter = makeFilter();
    const next = setTaskFilterSearch(filter, '  spaced  ');

    expect(next).not.toBe(filter);
    expect(next.search).toBe('  spaced  ');
    expect(filter.search).toBe('');
  });
});

describe('clearTaskFilter', () => {
  it('matches the default filter with fresh arrays', () => {
    const cleared = clearTaskFilter();

    expect(cleared).toEqual(DEFAULT_TASK_FILTER);
    expect(cleared.states).not.toBe(DEFAULT_TASK_FILTER.states);
    expect(cleared.tags).not.toBe(DEFAULT_TASK_FILTER.tags);
  });
});

describe('collectProjectTags', () => {
  it('collects task tags plus selected tags as sorted unique values', () => {
    const tasks = [task({ tags: [] }), task({ tags: ['bug'] }), task({ tags: ['urgent', 'bug'] })];

    expect(collectProjectTags(tasks, ['stale'])).toEqual(['bug', 'stale', 'urgent']);
  });

  it('normalizes case and whitespace and drops empties', () => {
    const tasks = [task({ tags: [' Bug ', '', 'URGENT '] })];

    expect(collectProjectTags(tasks)).toEqual(['bug', 'urgent']);
  });
});

describe('countTasksByTag', () => {
  it('counts tasks per tag', () => {
    const tasks = [
      task({ tags: [] }),
      task({ tags: [' Bug '] }),
      task({ tags: ['urgent', 'bug'] }),
    ];

    const counts = countTasksByTag(tasks);

    expect(counts.get('bug')).toBe(2);
    expect(counts.get('urgent')).toBe(1);
  });
});
