import { describe, expect, it } from 'vitest';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import type { BoardGroup } from './board';
import {
  buildBoard,
  dropPosition,
  groupKey,
  insertPosition,
  itemsFromGroups,
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
    ...overrides,
  };
}

function shape(groups: BoardGroup[]): [string | null, string[]][] {
  return groups.map((group) => [
    group.section?.id ?? null,
    group.tasks.map((candidate) => candidate.id),
  ]);
}

describe('groupKey', () => {
  it('uses the section id, with a sentinel for the unsectioned group', () => {
    expect(groupKey('backlog')).toBe('backlog');
    expect(groupKey(null)).toBe('no-section');
    expect(sectionIdFromGroup('backlog')).toBe('backlog');
    expect(sectionIdFromGroup('no-section')).toBeNull();
  });
});

describe('buildBoard', () => {
  it('keeps sections in snapshot order and includes empty ones', () => {
    const project = projectSnapshot({
      sections: [section('second', 'a1'), section('first', 'a0')],
    });

    expect(shape(buildBoard(project, false))).toEqual([
      ['second', []],
      ['first', []],
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

    expect(shape(buildBoard(project, false))).toEqual([
      ['backlog', ['one']],
      ['doing', ['two']],
    ]);
  });

  it('puts tasks without a section first', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'a0')],
      tasks: [
        task({ id: 'sectioned', title: 'Sectioned', section_id: 'backlog' }),
        task({ id: 'loose', title: 'Loose' }),
      ],
    });

    expect(shape(buildBoard(project, false))).toEqual([
      [null, ['loose']],
      ['backlog', ['sectioned']],
    ]);
  });

  it('omits the No section group when it has no tasks', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'a0')],
      tasks: [task({ id: 'sectioned', title: 'Sectioned', section_id: 'backlog' })],
    });

    expect(shape(buildBoard(project, false))).toEqual([['backlog', ['sectioned']]]);
  });

  it('hides closed tasks unless asked', () => {
    const project = projectSnapshot({
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

    expect(shape(buildBoard(project, false))).toEqual([['backlog', ['open']]]);
    expect(shape(buildBoard(project, true))).toEqual([['backlog', ['closed', 'open']]]);
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

    expect(shape(buildBoard(project, false))).toEqual([['backlog', ['critical', 'low']]]);
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

    expect(itemsFromGroups(buildBoard(project, false))).toEqual({
      backlog: ['a', 'b'],
      'no-section': ['c'],
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
