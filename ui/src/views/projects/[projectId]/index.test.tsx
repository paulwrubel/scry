// @vitest-environment jsdom
import type * as DndKitReact from '@dnd-kit/react';
import type * as DndKitSortable from '@dnd-kit/react/sortable';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import type { Replica } from '@/utils/replica';
import { BoardPage } from './index';

const dragIds = vi.hoisted(() => ({
  droppable: [] as string[],
  sortable: [] as string[],
}));

vi.mock('@dnd-kit/react', async (importOriginal) => {
  const actual = await importOriginal<typeof DndKitReact>();
  return {
    ...actual,
    useDroppable: (input: Parameters<typeof actual.useDroppable>[0]) => {
      dragIds.droppable.push(String(input.id));
      return actual.useDroppable(input);
    },
  };
});

vi.mock('@dnd-kit/react/sortable', async (importOriginal) => {
  const actual = await importOriginal<typeof DndKitSortable>();
  return {
    ...actual,
    useSortable: (input: Parameters<typeof actual.useSortable>[0]) => {
      dragIds.sortable.push(String(input.id));
      return actual.useSortable(input);
    },
  };
});

const CREATED_AT = '2026-01-01T00:00:00Z';

function section(id: string, name: string, position: string): Section {
  return {
    id,
    project_id: 'project-1',
    name,
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

function renderBoard(
  project: ProjectSnapshot,
  enqueue: (operation: Operation) => Promise<void>,
  path = `/projects/${project.id}`,
) {
  const snapshot = {
    confirmed: { projects: [project] },
    visible: { projects: [project] },
    outbox: [],
    needsBootstrap: false,
    persisted: true,
    lastSeenSeq: 0,
  };

  const context: ReplicaContextValue = {
    status: 'ready',
    replica: {
      getSnapshot: () => snapshot,
      subscribe: () => () => undefined,
      enqueue,
    } as unknown as Replica,
    error: undefined,
    retry: () => undefined,
  };

  render(
    <ReplicaContext.Provider value={context}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route path="/projects/:projectId" element={<BoardPage />} />
        </Routes>
      </MemoryRouter>
    </ReplicaContext.Provider>,
  );
}

afterEach(cleanup);

describe('drag ids', () => {
  it('gives the section sortable an id distinct from its item drop target', () => {
    dragIds.droppable.length = 0;
    dragIds.sortable.length = 0;

    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    expect(dragIds.droppable).toContain('backlog');
    expect(dragIds.sortable).not.toContain('backlog');
  });
});

describe('BoardPage', () => {
  it('renders a column per section and the tasks inside', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0'), section('doing', 'Doing', 'a1')],
      tasks: [
        task({ id: 'one', title: 'Write tests', section_id: 'backlog' }),
        task({ id: 'two', title: 'Ship board', section_id: 'doing' }),
      ],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    expect(screen.getByRole('heading', { name: 'Backlog' })).toBeDefined();
    expect(screen.getByRole('heading', { name: 'Doing' })).toBeDefined();
    expect(screen.getByText('Write tests')).toBeDefined();
    expect(screen.getByText('Ship board')).toBeDefined();
  });

  it('keeps empty sections visible', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    expect(screen.queryByText('No tasks.')).toBeNull();
    expect(screen.getByRole('button', { name: 'Add task' })).toBeDefined();
  });

  it('shows open and closed tasks by default', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [
        task({ id: 'open', title: 'Open task', section_id: 'backlog', position: 'a1' }),
        task({
          id: 'closed',
          title: 'Closed task',
          section_id: 'backlog',
          position: 'a0',
          state: 'closed',
        }),
      ],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    expect(screen.getByText('Open task')).toBeDefined();
    expect(screen.getByText('Closed task')).toBeDefined();
  });

  it('hides closed tasks when Open only is selected', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [
        task({ id: 'open', title: 'Open task', section_id: 'backlog', position: 'a1' }),
        task({
          id: 'closed',
          title: 'Closed task',
          section_id: 'backlog',
          position: 'a0',
          state: 'closed',
        }),
      ],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    fireEvent.click(screen.getByRole('button', { name: 'Filter tasks' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Open' }));

    expect(screen.getByText('Open task')).toBeDefined();
    expect(screen.queryByText('Closed task')).toBeNull();
  });

  it('shows only closed tasks when Closed is the only status', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [
        task({ id: 'open', title: 'Open task', section_id: 'backlog', position: 'a1' }),
        task({
          id: 'closed',
          title: 'Closed task',
          section_id: 'backlog',
          position: 'a0',
          state: 'closed',
        }),
      ],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    fireEvent.click(screen.getByRole('button', { name: 'Filter tasks' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Closed' }));

    expect(screen.getByText('Closed task')).toBeDefined();
    expect(screen.queryByText('Open task')).toBeNull();
  });

  it('defaults the status filter to All', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'open', title: 'Open task', section_id: 'backlog', position: 'a1' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    fireEvent.click(screen.getByRole('button', { name: 'Filter tasks' }));

    expect((screen.getByRole('radio', { name: 'All' }) as HTMLInputElement).checked).toBe(true);
    expect((screen.getByRole('radio', { name: 'Open' }) as HTMLInputElement).checked).toBe(false);
    expect((screen.getByRole('radio', { name: 'Closed' }) as HTMLInputElement).checked).toBe(false);
  });

  it('restores every task when All is selected', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [
        task({ id: 'open', title: 'Open task', section_id: 'backlog', position: 'a1' }),
        task({
          id: 'closed',
          title: 'Closed task',
          section_id: 'backlog',
          position: 'a0',
          state: 'closed',
        }),
      ],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    fireEvent.click(screen.getByRole('button', { name: 'Filter tasks' }));
    fireEvent.click(screen.getByRole('radio', { name: 'Open' }));

    expect(screen.queryByText('Closed task')).toBeNull();

    fireEvent.click(screen.getByRole('radio', { name: 'All' }));

    expect(screen.getByText('Open task')).toBeDefined();
    expect(screen.getByText('Closed task')).toBeDefined();
  });

  it('reads the closed-only status filter from the URL', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [
        task({ id: 'open', title: 'Open task', section_id: 'backlog', position: 'a1' }),
        task({
          id: 'closed',
          title: 'Closed task',
          section_id: 'backlog',
          position: 'a0',
          state: 'closed',
        }),
      ],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
      '/projects/project-1?state=closed',
    );

    expect(screen.getByText('Closed task')).toBeDefined();
    expect(screen.queryByText('Open task')).toBeNull();
  });

  it('filters tasks by tag', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [
        task({
          id: 'bug',
          title: 'Bug task',
          section_id: 'backlog',
          position: 'a0',
          tags: ['bug'],
        }),
        task({
          id: 'feature',
          title: 'Feature task',
          section_id: 'backlog',
          position: 'a1',
          tags: ['feature'],
        }),
      ],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    fireEvent.click(screen.getByRole('button', { name: 'Filter tasks' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'bug' }));

    expect(screen.getByText('Bug task')).toBeDefined();
    expect(screen.queryByText('Feature task')).toBeNull();
  });

  it('filters tasks by search', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [
        task({ id: 'tests', title: 'Write tests', section_id: 'backlog' }),
        task({ id: 'board', title: 'Ship board', section_id: 'backlog', position: 'a1' }),
      ],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    fireEvent.click(screen.getByRole('button', { name: 'Filter tasks' }));
    fireEvent.change(screen.getByLabelText('Filter by text'), {
      target: { value: 'ship' },
    });

    expect(screen.getByText('Ship board')).toBeDefined();
    expect(screen.queryByText('Write tests')).toBeNull();
  });

  it('reads the status filter from the URL', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [
        task({ id: 'open', title: 'Open task', section_id: 'backlog', position: 'a1' }),
        task({
          id: 'closed',
          title: 'Closed task',
          section_id: 'backlog',
          position: 'a0',
          state: 'closed',
        }),
      ],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
      '/projects/project-1?state=open',
    );

    expect(screen.getByText('Open task')).toBeDefined();
    expect(screen.queryByText('Closed task')).toBeNull();
  });

  it('reads tag filters from the URL', () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [
        task({
          id: 'bug',
          title: 'Bug task',
          section_id: 'backlog',
          position: 'a0',
          tags: ['bug'],
        }),
        task({
          id: 'feature',
          title: 'Feature task',
          section_id: 'backlog',
          position: 'a1',
          tags: ['feature'],
        }),
      ],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
      '/projects/project-1?tags=bug',
    );

    expect(screen.getByText('Bug task')).toBeDefined();
    expect(screen.queryByText('Feature task')).toBeNull();
  });

  it('explains that filters disable drag-and-drop', () => {
    const project = projectSnapshot({
      task_sorting_mode: 'manual',
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'open', title: 'Open task', section_id: 'backlog' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
      '/projects/project-1?state=open',
    );

    expect(screen.getByText('task drag-and-drop disabled due to applied filters')).toBeDefined();
  });

  it('explains that a non-manual sort mode disables drag-and-drop', () => {
    const project = projectSnapshot({
      task_sorting_mode: 'priority',
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'open', title: 'Open task', section_id: 'backlog' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    expect(screen.getByText('task drag-and-drop disabled due to sort mode')).toBeDefined();
  });

  it('explains when both sort mode and filters disable drag-and-drop', () => {
    const project = projectSnapshot({
      task_sorting_mode: 'priority',
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'open', title: 'Open task', section_id: 'backlog' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
      '/projects/project-1?state=open',
    );

    expect(
      screen.getByText('task drag-and-drop disabled due to sort mode and applied filters'),
    ).toBeDefined();
  });

  it('does not show the drag-and-drop warning for manual sort with no filters', () => {
    const project = projectSnapshot({
      task_sorting_mode: 'manual',
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'open', title: 'Open task', section_id: 'backlog' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    expect(screen.queryByText(/task drag-and-drop disabled/)).toBeNull();
  });

  it('orders the toolbar warning before filter, sort, and show priority', () => {
    const project = projectSnapshot({
      task_sorting_mode: 'manual',
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'open', title: 'Open task', section_id: 'backlog' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
      '/projects/project-1?state=open',
    );

    const warning = screen.getByText('task drag-and-drop disabled due to applied filters');
    const filter = screen.getByRole('button', { name: 'Filter tasks' });
    const sort = screen.getByRole('combobox', { name: 'Task sorting' });
    const priority = screen.getByRole('switch', { name: 'Show priority' });

    expect(warning.compareDocumentPosition(filter) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(filter.compareDocumentPosition(sort) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(sort.compareDocumentPosition(priority) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(warning.className).toContain('text-yellow-500/80');
  });

  it('enqueues a create_task operation in the section', async () => {
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'one', title: 'Existing', section_id: 'backlog', position: 'a1' })],
    });
    const enqueue = vi.fn(async (_operation: Operation) => undefined);

    renderBoard(project, enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText('Task title'), {
      target: { value: '  New task  ' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Add' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledTimes(1);
    });

    expect(enqueue).toHaveBeenCalledWith(
      expect.objectContaining({
        operation_version: 1,
        kind: 'create_task',
        project_id: project.id,
        title: 'New task',
        description: null,
        priority: 'medium',
        section_id: 'backlog',
        position: 'a2',
        state: 'open',
        tags: [],
      }),
    );

    const operation = enqueue.mock.calls[0][0];
    expect(operation.kind).toBe('create_task');
    if (operation.kind !== 'create_task') {
      throw new Error('expected a create_task operation');
    }
    expect(typeof operation.operation_id).toBe('string');
    expect(typeof operation.id).toBe('string');
    expect(operation.operation_id).not.toBe(operation.id);
    expect(operation.created_at).toMatch(/Z$/);
  });

  it('does not offer create for tasks without a section', () => {
    const project = projectSnapshot({
      tasks: [task({ id: 'loose', title: 'Loose task' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    expect(screen.getByRole('heading', { name: 'No section' })).toBeDefined();
    expect(screen.queryByRole('button', { name: 'Add task' })).toBeNull();
  });

  it('shows the not-found alert for an unknown project', () => {
    const project = projectSnapshot();

    renderBoard(
      project,
      vi.fn(async () => undefined),
      '/projects/missing',
    );

    expect(screen.getByText(/Project not found/)).toBeDefined();
  });

  it('offers to add a section for a project with no content', () => {
    renderBoard(
      projectSnapshot(),
      vi.fn(async () => undefined),
    );

    expect(screen.getByRole('button', { name: 'Add section' })).toBeDefined();
  });

  it('enqueues a section color change', () => {
    const enqueue = vi.fn(async () => undefined);
    const project = projectSnapshot({
      sections: [section('backlog', 'Backlog', 'a0')],
    });

    renderBoard(project, enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Set section color' }));
    fireEvent.click(screen.getByRole('button', { name: 'Magenta' }));

    expect(enqueue).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'update_section',
        id: 'backlog',
        name: null,
        color: { set: 'magenta' },
      }),
    );
  });
});
