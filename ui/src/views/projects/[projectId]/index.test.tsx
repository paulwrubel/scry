// @vitest-environment jsdom
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

  it('hides closed tasks until the filter is on', () => {
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

    expect(screen.queryByText('Closed task')).toBeNull();

    fireEvent.click(screen.getByRole('switch', { name: 'Show closed' }));

    expect(screen.getByText('Closed task')).toBeDefined();
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
});
