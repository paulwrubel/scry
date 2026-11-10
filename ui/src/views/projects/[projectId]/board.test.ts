import { describe, expect, it } from 'vitest';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import { DEFAULT_TASK_FILTER } from '@/utils/taskFilter';
import type { TaskFilter } from '@/utils/taskFilter';
import type { BoardGroup } from './board';
import {
  buildBoard,
  CLOSED_NO_SECTION_GROUP,
  dropPosition,
  groupKey,
  insertPosition,
  isNoSectionGroupKey,
  itemsFromGroups,
  OPEN_NO_SECTION_GROUP,
  planSectionMove,
  sectionIdFromGroup,
  sectionTasks,
} from './board';

const CREATED_AT = '2026-01-01T00:00:00Z';

function section(id: string, position: string): Section {
  return {
    id,
    project_id: 'project-1',
    name: id,
    position,
    color: null,
    created_at: CREATED_AT,
  };
}

function task(overrides: Partial<Task> & Pick<Task, 'id' | 'title'>): Task {
  return {
    project_id: 'project-1',
    description: null,
    priority: 'medium',
    section_id: null,
    position: 'a0',
    state: 'open',
    tags: [],
    created_at: CREATED_AT,
    ...overrides,
  };
}

function projectSnapshot(overrides: Partial<ProjectSnapshot> = {}): ProjectSnapshot {
  return {
    id: 'project-1',
    name: 'Project',
    description: null,
    task_sorting_mode: 'manual',
    show_priority: false,
    created_at: CREATED_AT,
    sections: [],
    tasks: [],
    notes: [],
    automations: [],
    ...overrides,
  };
}

function filter(overrides: Partial<TaskFilter> = {}): TaskFilter {
  return { states: [...DEFAULT_TASK_FILTER.states], tags: [], search: '', ...overrides };
}

function shape(groups: BoardGroup[]): [string, string[]][] {
  return groups.map((group) => [groupKey(group), group.tasks.map((candidate) => candidate.id)]);
}

describe('groupKey', () => {
  it('uses the section id for section groups', () => {
    expect(groupKey({ section: section('backlog', 'a0'), state: null })).toBe('backlog');
  });

  it('splits section-less groups by state', () => {
    expect(groupKey({ section: null, state: 'open' })).toBe(OPEN_NO_SECTION_GROUP);
    expect(groupKey({ section: null, state: 'closed' })).toBe(CLOSED_NO_SECTION_GROUP);
  });

  it('maps both section-less keys back to a null section id', () => {
    expect(sectionIdFromGroup('backlog')).toBe('backlog');
    expect(sectionIdFromGroup(OPEN_NO_SECTION_GROUP)).toBeNull();
    expect(sectionIdFromGroup(CLOSED_NO_SECTION_GROUP)).toBeNull();
    expect(isNoSectionGroupKey(OPEN_NO_SECTION_GROUP)).toBe(true);
    expect(isNoSectionGroupKey(CLOSED_NO_SECTION_GROUP)).toBe(true);
    expect(isNoSectionGroupKey('backlog')).toBe(false);
  });
});

describe('buildBoard', () => {
  it('orders sections by position and includes empty ones', () => {
    const project = projectSnapshot({
      sections: [section('second', 'a1'), section('first', 'a0')],
    });

    expect(shape(buildBoard(project, DEFAULT_TASK_FILTER))).toEqual([
      ['first', []],
      ['second', []],
    ]);
  });

  it('groups tasks into their section', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'a0'), section('doing', 'a1')],
      tasks: [
        task({ id: 'one', title: 'One', section_id: 'backlog', position: 'a0' }),
        task({ id: 'two', title: 'Two', section_id: 'doing', position: 'a0' }),
      ],
    });

    expect(shape(buildBoard(project, DEFAULT_TASK_FILTER))).toEqual([
      ['backlog', ['one']],
      ['doing', ['two']],
    ]);
  });

  it('puts open tasks without a section first', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'a0')],
      tasks: [
        task({ id: 'sectioned', title: 'Sectioned', section_id: 'backlog' }),
        task({ id: 'loose', title: 'Loose' }),
      ],
    });

    expect(shape(buildBoard(project, DEFAULT_TASK_FILTER))).toEqual([
      ['no-section-open', ['loose']],
      ['backlog', ['sectioned']],
    ]);
  });

  it('omits the No section group when it has no tasks', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'a0')],
      tasks: [task({ id: 'sectioned', title: 'Sectioned', section_id: 'backlog' })],
    });

    expect(shape(buildBoard(project, DEFAULT_TASK_FILTER))).toEqual([['backlog', ['sectioned']]]);
  });

  it('puts closed tasks without a section last', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'a0')],
      tasks: [
        task({ id: 'done', title: 'Done', state: 'closed', position: 'a0' }),
        task({ id: 'loose', title: 'Loose', position: 'a1' }),
        task({ id: 'sectioned', title: 'Sectioned', section_id: 'backlog', position: 'a0' }),
      ],
    });

    expect(shape(buildBoard(project, DEFAULT_TASK_FILTER))).toEqual([
      [OPEN_NO_SECTION_GROUP, ['loose']],
      ['backlog', ['sectioned']],
      [CLOSED_NO_SECTION_GROUP, ['done']],
    ]);
  });

  it('drops each no-section group when the state filter hides it', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'a0')],
      tasks: [
        task({ id: 'done', title: 'Done', state: 'closed', position: 'a0' }),
        task({ id: 'loose', title: 'Loose', position: 'a1' }),
      ],
    });

    expect(shape(buildBoard(project, filter({ states: ['open'] })))).toEqual([
      [OPEN_NO_SECTION_GROUP, ['loose']],
      ['backlog', []],
    ]);
    expect(shape(buildBoard(project, filter({ states: ['closed'] })))).toEqual([
      ['backlog', []],
      [CLOSED_NO_SECTION_GROUP, ['done']],
    ]);
  });

  const statusProject = projectSnapshot({
    sections: [section('backlog', 'a0')],
    tasks: [
      task({ id: 'open', title: 'Open', section_id: 'backlog', position: 'a1' }),
      task({
        id: 'closed',
        title: 'Closed',
        section_id: 'backlog',
        position: 'a0',
        state: 'closed',
      }),
    ],
  });

  it('shows both open and closed tasks by default', () => {
    expect(shape(buildBoard(statusProject, DEFAULT_TASK_FILTER))).toEqual([
      ['backlog', ['closed', 'open']],
    ]);
  });

  it('shows only open tasks when closed is deselected', () => {
    expect(shape(buildBoard(statusProject, filter({ states: ['open'] })))).toEqual([
      ['backlog', ['open']],
    ]);
  });

  it('shows only closed tasks when open is deselected', () => {
    expect(shape(buildBoard(statusProject, filter({ states: ['closed'] })))).toEqual([
      ['backlog', ['closed']],
    ]);
  });

  it('orders every group by the project sorting mode', () => {
    const project = projectSnapshot({
      task_sorting_mode: 'priority',
      sections: [section('backlog', 'a0')],
      tasks: [
        task({ id: 'low', title: 'Low', section_id: 'backlog', priority: 'low' }),
        task({ id: 'critical', title: 'Critical', section_id: 'backlog', priority: 'critical' }),
      ],
    });

    expect(shape(buildBoard(project, DEFAULT_TASK_FILTER))).toEqual([
      ['backlog', ['critical', 'low']],
    ]);
  });

  it('matches tasks with any selected tag', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'a0')],
      tasks: [
        task({ id: 'bug', title: 'Bug', section_id: 'backlog', position: 'a0', tags: ['bug'] }),
        task({
          id: 'urgent',
          title: 'Urgent',
          section_id: 'backlog',
          position: 'a1',
          tags: ['urgent'],
        }),
        task({
          id: 'feature',
          title: 'Feature',
          section_id: 'backlog',
          position: 'a2',
          tags: ['feature'],
        }),
      ],
    });

    expect(shape(buildBoard(project, filter({ tags: ['bug', 'urgent'] })))).toEqual([
      ['backlog', ['bug', 'urgent']],
    ]);
  });

  it('matches the title and description search', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'a0')],
      tasks: [
        task({ id: 'login', title: 'Fix login', section_id: 'backlog', position: 'a0' }),
        task({
          id: 'auth',
          title: 'Session',
          section_id: 'backlog',
          position: 'a1',
          description: 'Auth flow',
        }),
        task({ id: 'other', title: 'Docs', section_id: 'backlog', position: 'a2' }),
      ],
    });

    expect(shape(buildBoard(project, filter({ search: 'auth' })))).toEqual([['backlog', ['auth']]]);
  });
});

describe('itemsFromGroups', () => {
  it('maps each group to its task ids', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'a0')],
      tasks: [
        task({ id: 'a', title: 'A', section_id: 'backlog', position: 'a0' }),
        task({ id: 'b', title: 'B', section_id: 'backlog', position: 'a1' }),
        task({ id: 'c', title: 'C' }),
      ],
    });

    expect(itemsFromGroups(buildBoard(project, DEFAULT_TASK_FILTER))).toEqual({
      backlog: ['a', 'b'],
      [OPEN_NO_SECTION_GROUP]: ['c'],
    });
  });
});

describe('sectionTasks', () => {
  it('returns every task in the section in position order', () => {
    const project = projectSnapshot({
      tasks: [
        task({ id: 'a', title: 'A', section_id: 'backlog', position: 'a1' }),
        task({ id: 'b', title: 'B', section_id: 'backlog', position: 'a0' }),
        task({
          id: 'c',
          title: 'C',
          section_id: 'backlog',
          position: 'a2',
          state: 'closed',
        }),
        task({ id: 'd', title: 'D', section_id: 'doing', position: 'a0' }),
      ],
    });

    expect(sectionTasks(project, 'backlog').map((item) => item.id)).toEqual(['b', 'a', 'c']);
  });
});

describe('insertPosition', () => {
  const tasks = [
    task({ id: 'a', title: 'A', section_id: 'backlog', position: 'a0' }),
    task({ id: 'b', title: 'B', section_id: 'backlog', position: 'a1' }),
    task({ id: 'c', title: 'C', section_id: 'backlog', position: 'a2' }),
  ];

  it('inserts at the start', () => {
    expect(insertPosition(tasks, 0)).toBe('Zz');
  });

  it('inserts between tasks', () => {
    expect(insertPosition(tasks, 2)).toBe('a1V');
  });

  it('appends after the last task', () => {
    expect(insertPosition(tasks, 3)).toBe('a3');
  });

  it('starts at a0 for an empty section and clamps out-of-range indexes', () => {
    expect(insertPosition([], 0)).toBe('a0');
    expect(insertPosition(tasks, 99)).toBe('a3');
  });
});

describe('dropPosition', () => {
  const project = projectSnapshot({
    sections: [section('backlog', 'a0'), section('doing', 'a1')],
    tasks: [
      task({ id: 'a', title: 'A', section_id: 'backlog', position: 'a0' }),
      task({ id: 'b', title: 'B', section_id: 'backlog', position: 'a1' }),
      task({ id: 'c', title: 'C', section_id: 'backlog', position: 'a2' }),
      task({ id: 'd', title: 'D', section_id: 'doing', position: 'a0' }),
      task({ id: 'e', title: 'E', section_id: 'doing', position: 'a1' }),
    ],
  });

  it('reorders within a section', () => {
    expect(dropPosition(project, 'backlog', 'a', 'c')).toBe('a1V');
    expect(dropPosition(project, 'backlog', 'c', 'a')).toBe('Zz');
  });

  it('moves to another section before the next task', () => {
    expect(dropPosition(project, 'doing', 'a', 'e')).toBe('a0V');
  });

  it('appends when there is no next task', () => {
    expect(dropPosition(project, 'doing', 'a', null)).toBe('a2');
  });

  it('starts at a0 in an empty section', () => {
    const empty = projectSnapshot({
      sections: [section('empty', 'a0')],
      tasks: [task({ id: 'a', title: 'A', section_id: 'backlog', position: 'a0' })],
    });

    expect(dropPosition(empty, 'empty', 'a', null)).toBe('a0');
  });

  it('ignores closed tasks when finding the next neighbour', () => {
    const withClosed = projectSnapshot({
      tasks: [
        task({ id: 'a', title: 'A', section_id: 'backlog', position: 'a0' }),
        task({
          id: 'hidden',
          title: 'Hidden',
          section_id: 'backlog',
          position: 'a1',
          state: 'closed',
        }),
        task({ id: 'b', title: 'B', section_id: 'backlog', position: 'a2' }),
      ],
    });

    expect(dropPosition(withClosed, 'backlog', 'a', 'b')).toBe('a1V');
  });
});

describe('planSectionMove', () => {
  const sections = [section('a', 'a0'), section('b', 'a1'), section('c', 'a2')];

  it('moves a section down one slot', () => {
    expect(planSectionMove(sections, 0, 1)).toEqual({ sectionId: 'a', position: 'a1V' });
  });

  it('moves the dragged section, not the one it was dropped on', () => {
    expect(planSectionMove(sections, 0, 2)).toEqual({ sectionId: 'a', position: 'a3' });
  });

  it('moves a section to the front', () => {
    expect(planSectionMove(sections, 2, 0)).toEqual({ sectionId: 'c', position: 'Zz' });
  });

  it('moves a section up one slot', () => {
    expect(planSectionMove(sections, 2, 1)).toEqual({ sectionId: 'c', position: 'a0V' });
  });

  it('returns null when the index does not change', () => {
    expect(planSectionMove(sections, 1, 1)).toBeNull();
  });

  it('returns null for out-of-range indexes', () => {
    expect(planSectionMove(sections, -1, 1)).toBeNull();
    expect(planSectionMove(sections, 0, 3)).toBeNull();
  });
});
