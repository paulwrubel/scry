import { describe, expect, it } from 'vitest';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import { DEFAULT_TASK_FILTER } from '@/utils/taskFilter';
import type { TaskFilter } from '@/utils/taskFilter';
import type { BoardGroup, DragLayout, PriorityGroup, TaskSlot } from './board';
import {
  boundarySideForSlot,
  buildBoard,
  CLOSED_NO_SECTION_GROUP,
  dragPriority,
  dropPosition,
  dropPositionForPriority,
  groupKey,
  groupTasksByPriority,
  insertionIndexForPointer,
  insertPosition,
  isNoSectionGroupKey,
  itemsFromGroups,
  OPEN_NO_SECTION_GROUP,
  planSectionMove,
  previewAtPointer,
  previewAtPointerDetailed,
  sectionIdFromGroup,
  sectionTasks,
  soloPriorityHome,
  withGhostPriority,
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

describe('groupTasksByPriority', () => {
  it('chunks tasks of the same priority together', () => {
    const groups = groupTasksByPriority([
      task({ id: 'a', title: 'A', priority: 'high' }),
      task({ id: 'b', title: 'B', priority: 'high' }),
    ]);

    expect(groups.map((group) => [group.priority, group.tasks.map((item) => item.id)])).toEqual([
      ['high', ['a', 'b']],
    ]);
  });

  it('starts a new group when the priority changes', () => {
    const groups = groupTasksByPriority([
      task({ id: 'a', title: 'A', priority: 'high' }),
      task({ id: 'b', title: 'B', priority: 'low' }),
    ]);

    expect(groups.map((group) => [group.priority, group.tasks.map((item) => item.id)])).toEqual([
      ['high', ['a']],
      ['low', ['b']],
    ]);
  });

  it('keeps two separated runs of the same priority as separate groups', () => {
    const groups = groupTasksByPriority([
      task({ id: 'a', title: 'A', priority: 'high' }),
      task({ id: 'b', title: 'B', priority: 'low' }),
      task({ id: 'c', title: 'C', priority: 'high' }),
    ]);

    expect(groups.map((group) => [group.priority, group.tasks.map((item) => item.id)])).toEqual([
      ['high', ['a']],
      ['low', ['b']],
      ['high', ['c']],
    ]);
  });

  it('returns no groups for an empty list', () => {
    expect(groupTasksByPriority([])).toEqual([]);
  });

  it('groups by an override without mutating the task', () => {
    const b = task({ id: 'b', title: 'B', priority: 'high' });
    const groups = groupTasksByPriority([task({ id: 'a', title: 'A', priority: 'high' }), b], {
      b: 'medium',
    });

    expect(groups.map((group) => [group.priority, group.tasks.map((item) => item.id)])).toEqual([
      ['high', ['a']],
      ['medium', ['b']],
    ]);
    expect(b.priority).toBe('high');
  });
});

describe('withGhostPriority', () => {
  const bands: PriorityGroup[] = [
    { priority: 'high', tasks: [] },
    { priority: 'low', tasks: [] },
  ];

  it('inserts the ghost band at its sorted position', () => {
    expect(withGhostPriority(bands, 'medium').map((band) => band.priority)).toEqual([
      'high',
      'medium',
      'low',
    ]);
    expect(withGhostPriority(bands, 'minimal').map((band) => band.priority)).toEqual([
      'high',
      'low',
      'minimal',
    ]);
    expect(withGhostPriority(bands, 'critical').map((band) => band.priority)).toEqual([
      'critical',
      'high',
      'low',
    ]);
  });

  it('leaves the bands alone when the priority already exists', () => {
    expect(withGhostPriority(bands, 'high')).toBe(bands);
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

describe('dragPriority', () => {
  const project = projectSnapshot({
    tasks: [
      task({ id: 'a', title: 'A', priority: 'high', position: 'a0' }),
      task({ id: 'b', title: 'B', priority: 'high', position: 'a1' }),
      task({ id: 'c', title: 'C', priority: 'medium', position: 'a2' }),
      task({ id: 'd', title: 'D', priority: 'low', position: 'a0' }),
    ],
  });
  const baseItems = { backlog: ['a', 'b', 'c'], doing: ['d'] };

  it('returns null in manual mode', () => {
    expect(
      dragPriority(project, 'manual', baseItems, { backlog: ['a', 'c', 'b'], doing: ['d'] }, 'b'),
    ).toBeNull();
  });

  it('keeps the current priority when the drop is a no-op', () => {
    expect(dragPriority(project, 'priority', baseItems, baseItems, 'b')).toBe('high');
  });

  it('keeps a same-slot priority when it is one of the boundary neighbours', () => {
    const items = { backlog: ['a', 'b', 'c'] };

    expect(dragPriority(project, 'priority', items, items, 'b')).toBe('high');
  });

  it('resolves a shared boundary by the pointer side', () => {
    const mixed = projectSnapshot({
      tasks: [
        task({ id: 'a', title: 'A', priority: 'high', position: 'a0' }),
        task({ id: 'c', title: 'C', priority: 'medium', position: 'a1' }),
        task({ id: 'd', title: 'D', priority: 'low', position: 'a2' }),
      ],
    });
    const items = { backlog: ['a', 'c', 'd'] };

    expect(dragPriority(mixed, 'priority', items, items, 'c', 'above')).toBe('high');
    expect(dragPriority(mixed, 'priority', items, items, 'c', 'below')).toBe('low');
    expect(dragPriority(mixed, 'priority', items, items, 'c')).toBe('medium');
  });

  it('lets the pointer side override the current band at a same-slot boundary', () => {
    // b starts between a (high) and c (medium), in the same slot it would be
    // dropped in; the side still decides which band it joins
    expect(dragPriority(project, 'priority', baseItems, baseItems, 'b', 'above')).toBe('high');
    expect(dragPriority(project, 'priority', baseItems, baseItems, 'b', 'below')).toBe('medium');
  });

  it('uses the card below when the task moves down', () => {
    expect(
      dragPriority(project, 'priority', baseItems, { backlog: ['a', 'c', 'b'], doing: ['d'] }, 'b'),
    ).toBe('medium');
  });

  it('keeps the current priority at the bottom of the task group', () => {
    expect(
      dragPriority(project, 'priority', baseItems, { backlog: ['b', 'a', 'c'], doing: ['d'] }, 'a'),
    ).toBe('high');
  });

  it('keeps the current priority at the top of the task group', () => {
    const twoMedium = projectSnapshot({
      tasks: [
        task({ id: 'a', title: 'A', priority: 'high', position: 'a0' }),
        task({ id: 'c', title: 'C', priority: 'medium', position: 'a1' }),
        task({ id: 'e', title: 'E', priority: 'medium', position: 'a2' }),
      ],
    });

    expect(
      dragPriority(
        twoMedium,
        'priority',
        { backlog: ['a', 'c', 'e'] },
        { backlog: ['a', 'e', 'c'] },
        'e',
      ),
    ).toBe('medium');
  });

  it('uses the upper band for a task outside both boundary groups', () => {
    expect(
      dragPriority(
        project,
        'priority',
        baseItems,
        { backlog: ['a', 'b', 'd', 'c'], doing: [] },
        'd',
      ),
    ).toBe('high');
  });

  it('uses the destination column rule on a cross-group drop', () => {
    expect(
      dragPriority(project, 'priority', baseItems, { backlog: ['a', 'c'], doing: ['d', 'b'] }, 'b'),
    ).toBe('low');
  });
});

describe('boundarySideForSlot', () => {
  it('resolves above when the anchor is above the slot centre', () => {
    expect(boundarySideForSlot(77, 67)).toBe('above');
  });

  it('resolves below when the anchor is below the slot centre', () => {
    expect(boundarySideForSlot(77, 87)).toBe('below');
  });

  it('returns null exactly at the slot centre', () => {
    expect(boundarySideForSlot(77, 77)).toBeNull();
  });

  it('returns null when there is no slot', () => {
    expect(boundarySideForSlot(null, 77)).toBeNull();
  });
});

describe('priority boundary reachability', () => {
  it('reaches both sides without changing the insertion index', () => {
    const layout = dragLayout(
      {
        backlog: [taskSlot('a', 0), taskSlot('mid', 54), taskSlot('b', 108)],
      },
      46,
    );
    const items = { backlog: ['a', 'mid', 'b'] };

    const up = previewAtPointerDetailed(items, layout, 'backlog', 'mid', 67);
    const down = previewAtPointerDetailed(items, layout, 'backlog', 'mid', 87);

    // removing 'mid' leaves a 0-46 and b 54-100, so the slot centre is 77
    expect(up.index).toBe(1);
    expect(down.index).toBe(1);
    expect(up.slotCenter).toBe(77);
    expect(down.slotCenter).toBe(77);
    expect(boundarySideForSlot(up.slotCenter, 67)).toBe('above');
    expect(boundarySideForSlot(down.slotCenter, 87)).toBe('below');
  });

  it('ties at the centre of the slot the card was picked up from', () => {
    const layout = dragLayout(
      {
        backlog: [taskSlot('a', 0), taskSlot('mid', 54), taskSlot('b', 108)],
      },
      46,
    );

    const preview = previewAtPointerDetailed(
      { backlog: ['a', 'mid', 'b'] },
      layout,
      'backlog',
      'mid',
      77,
    );

    expect(preview.index).toBe(1);
    expect(boundarySideForSlot(preview.slotCenter, 77)).toBeNull();
  });
});

describe('soloPriorityHome', () => {
  const group: BoardGroup = {
    section: section('backlog', 'a0'),
    state: null,
    tasks: [task({ id: 'solo', title: 'Solo', priority: 'high', position: 'a0' })],
  };
  const layout = dragLayout({ backlog: [taskSlot('solo', 0)] }, 46);

  it('returns the original slot top when the task is alone in its priority', () => {
    expect(soloPriorityHome(group, 'solo', layout)).toEqual({ top: 0, height: 46 });
  });

  it('returns null when another task shares the priority', () => {
    const shared: BoardGroup = {
      ...group,
      tasks: [
        ...group.tasks,
        task({ id: 'other', title: 'Other', priority: 'high', position: 'a1' }),
      ],
    };
    const sharedLayout = dragLayout({ backlog: [taskSlot('solo', 0), taskSlot('other', 54)] }, 46);

    expect(soloPriorityHome(shared, 'solo', sharedLayout)).toBeNull();
  });

  it('returns null when the original slot is missing', () => {
    expect(soloPriorityHome(group, 'solo', dragLayout({ backlog: [] }, 46))).toBeNull();
  });
});

describe('dropPositionForPriority', () => {
  const project = projectSnapshot({
    sections: [section('backlog', 'a0')],
    tasks: [
      task({
        id: 'high-start',
        title: 'High start',
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
        id: 'high-end',
        title: 'High end',
        section_id: 'backlog',
        position: 'a2',
        priority: 'high',
      }),
    ],
  });

  it('inserts only among tasks of the target priority', () => {
    expect(dropPositionForPriority(project, 'backlog', 'high', 'dragged', 'high-end')).toBe('a1');
  });

  it('appends when there is no next task', () => {
    expect(dropPositionForPriority(project, 'backlog', 'high', 'dragged', null)).toBe('a3');
  });

  it('excludes the dragged task from the neighbour search', () => {
    const withDragged = projectSnapshot({
      sections: [section('backlog', 'a0')],
      tasks: [
        task({
          id: 'dragged',
          title: 'Dragged',
          section_id: 'backlog',
          position: 'a0',
          priority: 'high',
        }),
        task({
          id: 'next',
          title: 'Next',
          section_id: 'backlog',
          position: 'a1',
          priority: 'high',
        }),
      ],
    });

    expect(dropPositionForPriority(withDragged, 'backlog', 'high', 'dragged', 'next')).toBe('a0');
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
