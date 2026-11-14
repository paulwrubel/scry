// @vitest-environment jsdom
import type * as DndKitReact from '@dnd-kit/react';
import type * as DndKitSortable from '@dnd-kit/react/sortable';
import type * as MediaQuery from '@/hooks/useMediaQuery';
import { act, cleanup, fireEvent, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Operation } from '@/generated/Operation';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import { CREATED_AT, makeProject } from '@/test/fixtures';
import { renderBoard } from '@/test/renderBoard';

const dragIds = vi.hoisted(() => ({
  droppable: [] as string[],
  sortable: [] as string[],
}));

const viewport = vi.hoisted(() => ({ isDesktop: true }));

vi.mock('@/hooks/useMediaQuery', async (importOriginal) => {
  const actual = await importOriginal<typeof MediaQuery>();
  return { ...actual, useIsDesktop: () => viewport.isDesktop };
});

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

function section(id: string, name: string, position: string): Section {
  return {
    id,
    project_id: 'project-1',
    name,
    position,
    color: null,
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
  };
}

function task(overrides: Partial<Task> & Pick<Task, 'id' | 'title'>): Task {
  return {
    project_id: 'project-1',
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

afterEach(() => {
  viewport.isDesktop = true;
  localStorage.clear();
  cleanup();
});

describe('drag ids', () => {
  it('gives the section sortable an id distinct from its item drop target', () => {
    dragIds.droppable.length = 0;
    dragIds.sortable.length = 0;

    const project = makeProject({
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
    const project = makeProject({
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
    const project = makeProject({
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
    const project = makeProject({
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
    const project = makeProject({
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

  it('defaults the status filter to All', () => {
    const project = makeProject({
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

  it('filters tasks by tag', () => {
    const project = makeProject({
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
    const project = makeProject({
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
    const project = makeProject({
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

  it('orders tasks on the board from the sort URL param', () => {
    const project = makeProject({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [
        task({
          id: 'normal',
          title: 'Normal',
          section_id: 'backlog',
          position: 'a0',
          priority: 'normal',
        }),
        task({
          id: 'critical',
          title: 'Critical',
          section_id: 'backlog',
          position: 'a1',
          priority: 'critical',
        }),
      ],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
      '/projects/project-1?sort=priority',
    );

    const normal = screen.getByText('Normal');
    const critical = screen.getByText('Critical');

    expect(
      critical.compareDocumentPosition(normal) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  it('updates the sort URL param without enqueueing an operation', () => {
    const enqueue = vi.fn(async (_operation: Operation) => undefined);
    const project = makeProject({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'one', title: 'One', section_id: 'backlog' })],
    });

    renderBoard(project, enqueue);

    const sort = screen.getByRole('combobox', { name: 'Task sorting' }) as HTMLSelectElement;
    fireEvent.change(sort, { target: { value: 'priority' } });

    expect(screen.getByTestId('search').textContent).toBe('sort=priority');
    expect(sort.value).toBe('priority');
    expect(enqueue).not.toHaveBeenCalledWith(expect.objectContaining({ kind: 'update_project' }));
    expect(enqueue).not.toHaveBeenCalled();
  });

  it('removes the sort param when switching back to manual while preserving other params', () => {
    const project = makeProject({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'one', title: 'One', section_id: 'backlog' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
      '/projects/project-1?sort=priority&state=open',
    );

    const sort = screen.getByRole('combobox', { name: 'Task sorting' }) as HTMLSelectElement;
    fireEvent.change(sort, { target: { value: 'manual' } });

    expect(screen.getByTestId('search').textContent).toBe('state=open');
    expect(sort.value).toBe('manual');
  });

  it('falls back to manual for an invalid sort param', () => {
    const project = makeProject({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'one', title: 'One', section_id: 'backlog' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
      '/projects/project-1?sort=banana',
    );

    const sort = screen.getByRole('combobox', { name: 'Task sorting' }) as HTMLSelectElement;
    expect(sort.value).toBe('manual');
  });

  it('enqueues a create_task operation in the section', async () => {
    const project = makeProject({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'one', title: 'Existing', section_id: 'backlog', position: 'a1' })],
    });
    const enqueue = vi.fn(async (_operation: Operation) => undefined);

    renderBoard(project, enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText('Task title or GitHub URL'), {
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
        priority: 'normal',
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

  it('offers create in the Open No Section column', async () => {
    const project = makeProject({
      tasks: [task({ id: 'loose', title: 'Loose task' })],
    });
    const enqueue = vi.fn(async (_operation: Operation) => undefined);

    renderBoard(project, enqueue);

    expect(screen.getByRole('heading', { name: 'Open (No Section)' })).toBeDefined();

    fireEvent.click(screen.getByRole('button', { name: 'Add task' }));
    fireEvent.change(screen.getByLabelText('Task title or GitHub URL'), {
      target: { value: 'New loose task' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Add' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledTimes(1);
    });

    expect(enqueue).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'create_task',
        section_id: null,
        state: 'open',
        title: 'New loose task',
      }),
    );
  });

  it('does not offer create in the Closed No Section column', () => {
    const project = makeProject({
      tasks: [task({ id: 'done', title: 'Done task', state: 'closed' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    expect(screen.getByRole('heading', { name: 'Closed (No Section)' })).toBeDefined();
    expect(screen.queryByRole('button', { name: 'Add task' })).toBeNull();
  });

  it('shows the not-found alert for an unknown project', () => {
    const project = makeProject();

    renderBoard(
      project,
      vi.fn(async () => undefined),
      '/projects/missing',
    );

    expect(screen.getByText(/Project not found/)).toBeDefined();
  });

  it('offers to add a section for a project with no content', () => {
    renderBoard(
      makeProject(),
      vi.fn(async () => undefined),
    );

    expect(screen.getByRole('button', { name: 'Add section' })).toBeDefined();
  });

  it('enqueues a section color change', () => {
    const enqueue = vi.fn(async () => undefined);
    const project = makeProject({
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

describe('show priority', () => {
  it('toggles priority display from the mobile toolbar button', () => {
    viewport.isDesktop = false;
    const project = makeProject({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'one', title: 'One', section_id: 'backlog', priority: 'elevated' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
      '/projects/project-1?state=open',
    );

    expect(screen.queryByRole('img', { name: 'elevated priority' })).toBeNull();

    const toggle = screen.getByRole('switch', { name: 'Show priority' });
    expect(toggle.getAttribute('aria-checked')).toBe('false');

    fireEvent.click(toggle);

    expect(screen.getByTestId('search').textContent).toBe('state=open&show_priority=true');
    expect(toggle.getAttribute('aria-checked')).toBe('true');
    expect(screen.getByRole('img', { name: 'elevated priority' })).toBeDefined();

    fireEvent.click(toggle);

    expect(screen.getByTestId('search').textContent).toBe('state=open');
    expect(toggle.getAttribute('aria-checked')).toBe('false');
    expect(screen.queryByRole('img', { name: 'elevated priority' })).toBeNull();
  });

  it('keeps the labelled switch on desktop', () => {
    const project = makeProject({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'one', title: 'One', section_id: 'backlog', priority: 'elevated' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
      '/projects/project-1?show_priority=true',
    );

    const toggle = screen.getByRole('switch', { name: 'Show priority' });
    expect(toggle.getAttribute('aria-checked')).toBe('true');
    expect(screen.queryByRole('button', { name: 'Show priority' })).toBeNull();
    expect(screen.getByRole('img', { name: 'elevated priority' })).toBeDefined();
  });
});

describe('collapsible sections', () => {
  function collapseButton(name: string) {
    return screen.getByRole('button', { name });
  }

  it('collapses and expands a section on mobile', async () => {
    viewport.isDesktop = false;
    const project = makeProject({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'one', title: 'Write tests', section_id: 'backlog' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    expect(screen.getByText('Write tests')).toBeDefined();

    fireEvent.click(collapseButton('Collapse Backlog'));

    expect(collapseButton('Expand Backlog')).toBeDefined();
    await waitFor(() => expect(screen.queryByText('Write tests')).toBeNull());

    fireEvent.click(collapseButton('Expand Backlog'));

    expect(collapseButton('Collapse Backlog')).toBeDefined();
    await waitFor(() => expect(screen.getByText('Write tests')).toBeDefined());
  });

  it('toggles twice when two taps land before the commit', () => {
    viewport.isDesktop = false;
    const project = makeProject({
      sections: [section('backlog', 'Backlog', 'a0')],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    const button = collapseButton('Collapse Backlog');
    act(() => {
      button.click();
      button.click();
    });

    expect(collapseButton('Collapse Backlog')).toBeDefined();
  });

  it('toggles from the whole header row', () => {
    viewport.isDesktop = false;
    const project = makeProject({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'one', title: 'Write tests', section_id: 'backlog' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    fireEvent.click(screen.getByRole('heading', { name: 'Backlog' }));

    expect(collapseButton('Expand Backlog')).toBeDefined();

    fireEvent.click(collapseButton('Expand Backlog'));

    expect(collapseButton('Collapse Backlog')).toBeDefined();
  });

  it('collapses a no-section column on mobile', async () => {
    viewport.isDesktop = false;
    const project = makeProject({
      tasks: [task({ id: 'loose', title: 'Loose task' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    fireEvent.click(collapseButton('Collapse Open (No Section)'));

    expect(collapseButton('Expand Open (No Section)')).toBeDefined();
    await waitFor(() => expect(document.getElementById('no-section-open-tasks')).toBeNull());
  });

  it('renders no collapse controls on desktop', () => {
    const project = makeProject({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'one', title: 'Write tests', section_id: 'backlog' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    expect(screen.queryByRole('button', { name: /^Collapse / })).toBeNull();
    expect(screen.getByText('Write tests')).toBeDefined();
  });

  it('remembers a collapsed section across renders', () => {
    viewport.isDesktop = false;
    const project = makeProject({
      sections: [section('backlog', 'Backlog', 'a0')],
      tasks: [task({ id: 'one', title: 'Write tests', section_id: 'backlog' })],
    });

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );
    fireEvent.click(collapseButton('Collapse Backlog'));

    cleanup();

    renderBoard(
      project,
      vi.fn(async () => undefined),
    );

    expect(collapseButton('Expand Backlog')).toBeDefined();
  });
});

describe('mobile quick add', () => {
  it('hides inline add buttons and creates a No Section task from the FAB', async () => {
    viewport.isDesktop = false;
    const enqueue = vi.fn(async (_operation: Operation) => undefined);
    const project = makeProject({
      sections: [section('backlog', 'Backlog', 'a0')],
    });

    renderBoard(project, enqueue);

    const addButtons = screen.getAllByRole('button', { name: 'Add task' });
    expect(addButtons).toHaveLength(1);

    fireEvent.click(addButtons[0]);
    fireEvent.change(screen.getByLabelText('Task title or GitHub URL'), {
      target: { value: 'Mobile task' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Add' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledTimes(1);
    });

    expect(enqueue).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'create_task',
        title: 'Mobile task',
        section_id: null,
      }),
    );
  });
});
