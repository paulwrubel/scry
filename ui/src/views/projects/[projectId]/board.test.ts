import { describe, expect, it } from 'vitest';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import type { BoardGroup } from './board';
import { buildBoard, nextTaskPosition } from './board';

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

  it('appends tasks without a section last', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'a0')],
      tasks: [
        task({ id: 'sectioned', title: 'Sectioned', section_id: 'backlog' }),
        task({ id: 'loose', title: 'Loose' }),
      ],
    });

    expect(shape(buildBoard(project, false))).toEqual([
      ['backlog', ['sectioned']],
      [null, ['loose']],
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

  it('interleaves closed tasks under the same sorting mode', () => {
    const project = projectSnapshot({
      task_sorting_mode: 'priority',
      sections: [section('backlog', 'a0')],
      tasks: [
        task({ id: 'open', title: 'Open', section_id: 'backlog', priority: 'low' }),
        task({
          id: 'closed',
          title: 'Closed',
          section_id: 'backlog',
          priority: 'critical',
          state: 'closed',
        }),
      ],
    });

    expect(shape(buildBoard(project, true))).toEqual([['backlog', ['closed', 'open']]]);
  });
});

describe('nextTaskPosition', () => {
  it('appends after the last task in the section', () => {
    const tasks = [
      task({ id: 'one', title: 'One', section_id: 'backlog', position: 'a0' }),
      task({ id: 'two', title: 'Two', section_id: 'backlog', position: 'a1' }),
    ];

    expect(nextTaskPosition(tasks, 'backlog')).toBe('a2');
  });

  it('ignores other sections and includes closed tasks', () => {
    const tasks = [
      task({ id: 'one', title: 'One', section_id: 'backlog', position: 'a0' }),
      task({
        id: 'closed',
        title: 'Closed',
        section_id: 'backlog',
        position: 'a2',
        state: 'closed',
      }),
      task({ id: 'other', title: 'Other', section_id: 'doing', position: 'a9' }),
    ];

    expect(nextTaskPosition(tasks, 'backlog')).toBe('a3');
  });

  it('starts at a0 for an empty section', () => {
    expect(nextTaskPosition([], 'backlog')).toBe('a0');
  });
});
