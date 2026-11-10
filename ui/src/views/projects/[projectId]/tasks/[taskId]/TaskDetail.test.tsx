// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Note } from '@/generated/Note';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import type { Replica } from '@/utils/replica';
import { TaskDetail } from './TaskDetail';

const PROJECT_ID = 'project-1';
const TASK_ID = 'task-1';
const CREATED_AT = '2026-01-01T00:00:00Z';

function section(id: string, name: string, position: string): Section {
  return {
    id,
    project_id: PROJECT_ID,
    name,
    position,
    color: null,
    created_at: CREATED_AT,
  };
}

function task(overrides: Partial<Task> & Pick<Task, 'id' | 'title'>): Task {
  return {
    project_id: PROJECT_ID,
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

function note(overrides: Partial<Note> & Pick<Note, 'id' | 'contents'>): Note {
  return {
    task_id: TASK_ID,
    created_at: CREATED_AT,
    ...overrides,
  };
}

function projectSnapshot(overrides: Partial<ProjectSnapshot> = {}): ProjectSnapshot {
  return {
    id: PROJECT_ID,
    name: 'Project',
    description: null,
    task_sorting_mode: 'manual',
    show_priority: false,
    created_at: CREATED_AT,
    sections: [section('backlog', 'Backlog', 'a0'), section('doing', 'Doing', 'a1')],
    tasks: [
      task({
        id: TASK_ID,
        title: 'Write tests',
        description: 'Details',
        priority: 'high',
        section_id: 'backlog',
        tags: ['one', 'two'],
      }),
      task({ id: 'task-2', title: 'Other task', section_id: 'doing', position: 'a4' }),
    ],
    notes: [note({ id: 'note-1', contents: 'A note' })],
    automations: [],
    ...overrides,
  };
}

function LocationProbe() {
  const location = useLocation();
  return <span data-testid="path">{location.pathname}</span>;
}

function renderTask(
  project: ProjectSnapshot,
  enqueue: (operation: Operation) => Promise<void>,
  onClose = vi.fn(),
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
      <MemoryRouter initialEntries={[`/projects/${PROJECT_ID}`]}>
        <LocationProbe />
        <TaskDetail project={project} task={project.tasks[0]} handle="abcd" onClose={onClose} />
      </MemoryRouter>
    </ReplicaContext.Provider>,
  );
}

afterEach(cleanup);

describe('TaskDetail', () => {
  it('shows the task fields and notes', () => {
    renderTask(
      projectSnapshot(),
      vi.fn(async () => undefined),
    );

    expect((screen.getByLabelText('Title') as HTMLInputElement).value).toBe('Write tests');
    expect((screen.getByLabelText('Description') as HTMLTextAreaElement).value).toBe('Details');
    expect((screen.getByLabelText('Priority') as HTMLSelectElement).value).toBe('high');
    expect((screen.getByLabelText('Section') as HTMLSelectElement).value).toBe('backlog');
    expect((screen.getByLabelText('Tags') as HTMLInputElement).value).toBe('one, two');
    expect(screen.getByText('abcd')).toBeDefined();
    expect(screen.getByText('A note')).toBeDefined();
  });

  it('rejects an empty title', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.change(screen.getByLabelText('Title'), { target: { value: '   ' } });

    expect(await screen.findByText('A task needs a title.')).toBeDefined();
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(enqueue).not.toHaveBeenCalled();
  });

  it('saves changed fields as one update operation', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'New title' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledTimes(1);
    });
    expect(enqueue).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'update_task',
        id: TASK_ID,
        title: 'New title',
        priority: null,
        tags: null,
        state: null,
      }),
    );
    expect(await screen.findByText('Saved locally')).toBeDefined();
  });

  it('moves the task when the section changes', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.change(screen.getByLabelText('Section'), { target: { value: 'doing' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledTimes(1);
    });
    expect(enqueue).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'move_task',
        id: TASK_ID,
        section_id: 'doing',
        position: 'a5',
      }),
    );
  });

  it('closes an open task', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Close task' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({ kind: 'update_task', id: TASK_ID, state: 'closed' }),
      );
    });
  });

  it('reopens a closed task', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(
      projectSnapshot({
        tasks: [
          task({
            id: TASK_ID,
            title: 'Closed task',
            state: 'closed',
            section_id: 'backlog',
          }),
        ],
      }),
      enqueue,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Reopen task' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({ kind: 'update_task', id: TASK_ID, state: 'open' }),
      );
    });
  });

  it('deletes the task after confirmation', async () => {
    const enqueue = vi.fn(async () => undefined);
    const onClose = vi.fn();
    renderTask(projectSnapshot(), enqueue, onClose);

    fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
    fireEvent.click(screen.getByRole('button', { name: 'Delete task' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({ kind: 'delete_task', id: TASK_ID }),
      );
    });
    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });

  it('adds a note', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.change(screen.getByLabelText('New note'), {
      target: { value: '  A new note  ' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Add note' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'create_note',
          task_id: TASK_ID,
          contents: 'A new note',
        }),
      );
    });
  });

  it('edits a note inline', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Edit note' }));
    fireEvent.change(screen.getByLabelText('Note contents'), {
      target: { value: 'Updated note' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save note' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'update_note',
          id: 'note-1',
          contents: 'Updated note',
        }),
      );
    });
  });

  it('deletes a note after two-step confirmation', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Delete note' }));
    fireEvent.click(screen.getByRole('button', { name: 'Confirm delete note' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({ kind: 'delete_note', id: 'note-1' }),
      );
    });
  });
});
