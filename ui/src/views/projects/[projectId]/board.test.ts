import { describe, expect, it } from 'vitest';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import { DEFAULT_TASK_FILTER } from '@/utils/taskFilter';
import type { TaskFilter } from '@/utils/taskFilter';
import type { BoardGroup, DragLayout, TaskSlot } from './board';
import {
  buildBoard,
  CLOSED_NO_SECTION_GROUP,
  dropPosition,
  dropPositionForPriority,
  groupKey,
  insertionIndexForPointer,
  insertPosition,
  isNoSectionGroupKey,
  itemsFromGroups,
  OPEN_NO_SECTION_GROUP,
  planSectionMove,
  previewAtPointer,
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
    external_identity: null,
    external_metadata: null,
    ...overrides,
  };
}

function projectSnapshot(overrides: Partial<ProjectSnapshot> = {}): ProjectSnapshot {
  return {
    id: 'project-1',
    name: 'Project',
    description: null,
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

function taskSlot(id: string, top: number, height = 46): TaskSlot {
  return { id, top, bottom: top + height };
}

function dragLayout(
  groups: Record<string, TaskSlot[]>,
  draggedHeight: number,
  gap = 8,
): DragLayout {
  return {
    groups: Object.fromEntries(Object.entries(groups).map(([key, slots]) => [key, { gap, slots }])),
    draggedHeight,
  };
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

    expect(shape(buildBoard(project, DEFAULT_TASK_FILTER, 'manual'))).toEqual([
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

    expect(shape(buildBoard(project, DEFAULT_TASK_FILTER, 'manual'))).toEqual([
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

    expect(shape(buildBoard(project, DEFAULT_TASK_FILTER, 'manual'))).toEqual([
      ['no-section-open', ['loose']],
      ['backlog', ['sectioned']],
    ]);
  });

  it('omits the No section group when it has no tasks', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'a0')],
      tasks: [task({ id: 'sectioned', title: 'Sectioned', section_id: 'backlog' })],
    });

    expect(shape(buildBoard(project, DEFAULT_TASK_FILTER, 'manual'))).toEqual([
      ['backlog', ['sectioned']],
    ]);
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

    expect(shape(buildBoard(project, DEFAULT_TASK_FILTER, 'manual'))).toEqual([
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

    expect(shape(buildBoard(project, filter({ states: ['open'] }), 'manual'))).toEqual([
      [OPEN_NO_SECTION_GROUP, ['loose']],
      ['backlog', []],
    ]);
    expect(shape(buildBoard(project, filter({ states: ['closed'] }), 'manual'))).toEqual([
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
    expect(shape(buildBoard(statusProject, DEFAULT_TASK_FILTER, 'manual'))).toEqual([
      ['backlog', ['closed', 'open']],
    ]);
  });

  it('shows only open tasks when closed is deselected', () => {
    expect(shape(buildBoard(statusProject, filter({ states: ['open'] }), 'manual'))).toEqual([
      ['backlog', ['open']],
    ]);
  });

  it('shows only closed tasks when open is deselected', () => {
    expect(shape(buildBoard(statusProject, filter({ states: ['closed'] }), 'manual'))).toEqual([
      ['backlog', ['closed']],
    ]);
  });

  it('orders every group by the explicit sort mode', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'a0')],
      tasks: [
        task({ id: 'low', title: 'Low', section_id: 'backlog', priority: 'low' }),
        task({ id: 'critical', title: 'Critical', section_id: 'backlog', priority: 'critical' }),
      ],
    });

    expect(shape(buildBoard(project, DEFAULT_TASK_FILTER, 'priority'))).toEqual([
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

    expect(shape(buildBoard(project, filter({ tags: ['bug', 'urgent'] }), 'manual'))).toEqual([
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

    expect(shape(buildBoard(project, filter({ search: 'auth' }), 'manual'))).toEqual([
      ['backlog', ['auth']],
    ]);
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

    expect(itemsFromGroups(buildBoard(project, DEFAULT_TASK_FILTER, 'manual'))).toEqual({
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

describe('dropPositionForPriority', () => {
  const project = projectSnapshot({
    sections: [section('backlog', 'a0'), section('doing', 'a1')],
    tasks: [
      task({
        id: 'high-a',
        title: 'High A',
        section_id: 'backlog',
        position: 'a0',
        priority: 'high',
      }),
      task({
        id: 'medium',
        title: 'Medium',
        section_id: 'backlog',
        position: 'a1',
        priority: 'medium',
      }),
      task({
        id: 'high-b',
        title: 'High B',
        section_id: 'backlog',
        position: 'a2',
        priority: 'high',
      }),
      task({
        id: 'doing-high',
        title: 'Doing high',
        section_id: 'doing',
        position: 'a0',
        priority: 'high',
      }),
    ],
  });

  it('inserts only among tasks of the same priority', () => {
    expect(dropPositionForPriority(project, 'backlog', 'high', 'dragged', 'high-b')).toBe('a1');
  });

  it('appends within the priority when the next visible task is another priority', () => {
    expect(dropPositionForPriority(project, 'backlog', 'high', 'dragged', 'medium')).toBe('a3');
  });

  it('finds the same priority in another section', () => {
    expect(dropPositionForPriority(project, 'doing', 'high', 'dragged', null)).toBe('a1');
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

describe('insertionIndexForPointer', () => {
  const slots = [taskSlot('a', 0), taskSlot('b', 54)];

  it('returns 0 for empty slots', () => {
    expect(insertionIndexForPointer([], 0, 46, 8)).toBe(0);
  });

  it('returns the nearest placeholder slot', () => {
    expect(insertionIndexForPointer(slots, 0, 46, 8)).toBe(0);
    expect(insertionIndexForPointer(slots, 80, 46, 8)).toBe(1);
    expect(insertionIndexForPointer(slots, 200, 46, 8)).toBe(2);
  });

  it('keeps a tall card above a slot whose center it already passed', () => {
    // placeholder centers are 42, 96 and 150, so 60 sits nearest the first one
    expect(insertionIndexForPointer(slots, 60, 84, 8)).toBe(0);
  });

  it('moves to the next slot once the anchor clears its center for a tall card', () => {
    expect(insertionIndexForPointer(slots, 100, 84, 8)).toBe(1);
  });

  it('keeps the lower index on a tie', () => {
    // 23 and 77 are equidistant from 50
    expect(insertionIndexForPointer(slots, 50, 46, 8)).toBe(0);
  });
});

describe('previewAtPointer', () => {
  const layout = dragLayout(
    {
      backlog: [taskSlot('a', 0), taskSlot('b', 54)],
      doing: [taskSlot('c', 0), taskSlot('d', 54)],
    },
    46,
  );

  it('moves a task to another group at the nearest slot', () => {
    const items = { backlog: ['a', 'b'], doing: ['c', 'd'] };

    expect(previewAtPointer(items, layout, 'doing', 'a', 80)).toEqual({
      backlog: ['b'],
      doing: ['c', 'a', 'd'],
    });
  });

  it('inserts at the start of the destination above every slot', () => {
    const items = { backlog: ['a', 'b'], doing: ['c', 'd'] };

    expect(previewAtPointer(items, layout, 'doing', 'a', 0)).toEqual({
      backlog: ['b'],
      doing: ['a', 'c', 'd'],
    });
  });

  it('appends below every destination slot', () => {
    const items = { backlog: ['a', 'b'], doing: ['c', 'd'] };

    expect(previewAtPointer(items, layout, 'doing', 'a', 300)).toEqual({
      backlog: ['b'],
      doing: ['c', 'd', 'a'],
    });
  });

  it('closes the gap left by a same-group drag', () => {
    const items = { backlog: ['a', 'b'] };
    const sameGroup = dragLayout({ backlog: [taskSlot('a', 0), taskSlot('b', 54)] }, 46);

    expect(previewAtPointer(items, sameGroup, 'backlog', 'a', 60)).toEqual({
      backlog: ['b', 'a'],
    });
  });

  it('returns the same object when the preview does not change', () => {
    const items = { backlog: ['a', 'b'] };
    const sameGroup = dragLayout({ backlog: [taskSlot('a', 0), taskSlot('b', 54)] }, 46);

    expect(previewAtPointer(items, sameGroup, 'backlog', 'a', 0)).toBe(items);
  });

  it('returns the same object for an unknown destination', () => {
    const items = { backlog: ['a', 'b'], doing: ['c', 'd'] };

    expect(previewAtPointer(items, layout, 'missing', 'a', 80)).toBe(items);
  });

  it('does not mutate the input items or the layout slots', () => {
    const items = { backlog: ['a', 'b'], doing: ['c', 'd'] };
    const mutable = dragLayout(
      {
        backlog: [taskSlot('a', 0), taskSlot('b', 54)],
        doing: [taskSlot('c', 0), taskSlot('d', 54)],
      },
      46,
    );

    previewAtPointer(items, mutable, 'doing', 'a', 80);

    expect(items).toEqual({ backlog: ['a', 'b'], doing: ['c', 'd'] });
    expect(mutable.groups.backlog.slots).toEqual([taskSlot('a', 0), taskSlot('b', 54)]);
    expect(mutable.groups.doing.slots).toEqual([taskSlot('c', 0), taskSlot('d', 54)]);
  });
});

describe('previewAtPointer with a priority scope', () => {
  const layout = dragLayout(
    {
      backlog: [
        taskSlot('high-a', 0),
        taskSlot('high-b', 54),
        taskSlot('medium-a', 108),
        taskSlot('medium-b', 162),
      ],
      doing: [taskSlot('high-c', 0), taskSlot('low-a', 54)],
    },
    46,
  );
  const scope = {
    priority: 'medium' as const,
    priorityOf: (id: string) => {
      if (id.startsWith('high')) {
        return 'high' as const;
      }
      if (id.startsWith('low')) {
        return 'low' as const;
      }
      return 'medium' as const;
    },
  };

  it('inserts at the top of its own band when hovered over the band above', () => {
    const items = { backlog: ['high-a', 'high-b', 'medium-a', 'medium-b'], doing: [] };

    expect(previewAtPointer(items, layout, 'backlog', 'medium-b', 0, scope)).toEqual({
      backlog: ['high-a', 'high-b', 'medium-b', 'medium-a'],
      doing: [],
    });
  });

  it('never opens a gap between tasks of another priority', () => {
    const items = { backlog: ['high-a', 'high-b', 'medium-a', 'medium-b'], doing: [] };

    expect(previewAtPointer(items, layout, 'backlog', 'medium-b', 80, scope)).toEqual({
      backlog: ['high-a', 'high-b', 'medium-b', 'medium-a'],
      doing: [],
    });
  });

  it('places a new priority band by rank when the destination has no peer', () => {
    const items = { backlog: ['medium-a'], doing: ['high-c', 'low-a'] };

    expect(previewAtPointer(items, layout, 'doing', 'medium-a', 80, scope)).toEqual({
      backlog: [],
      doing: ['high-c', 'medium-a', 'low-a'],
    });
  });
});
