// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Comment } from '@/generated/Comment';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { ExternalMetadata } from '@/generated/ExternalMetadata';
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

const GITHUB_IDENTITY: ExternalIdentity = {
  provider: 'github',
  owner: 'acme',
  repo: 'widgets',
  number: 123,
};

function metadata(overrides: Partial<ExternalMetadata> = {}): ExternalMetadata {
  return {
    provider: 'github',
    title: 'Issue title',
    description: 'Issue body',
    kind: 'issue',
    state: 'open',
    reason: null,
    draft: false,
    updated_at: '2026-02-01T12:00:00Z',
    error: null,
    ...overrides,
  };
}

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
    priority: 'normal',
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

function comment(overrides: Partial<Comment> & Pick<Comment, 'id' | 'contents'>): Comment {
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
    created_at: CREATED_AT,
    sections: [section('backlog', 'Backlog', 'a0'), section('doing', 'Doing', 'a1')],
    tasks: [
      task({
        id: TASK_ID,
        title: 'Write tests',
        description: 'Details',
        priority: 'elevated',
        section_id: 'backlog',
        tags: ['one', 'two'],
      }),
      task({ id: 'task-2', title: 'Other task', section_id: 'doing', position: 'a4' }),
    ],
    comments: [comment({ id: 'comment-1', contents: 'A comment' })],
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
  it('shows the task in view mode', () => {
    renderTask(
      projectSnapshot(),
      vi.fn(async () => undefined),
    );

    expect(screen.getByText('Details')).toBeDefined();
    expect((screen.getByLabelText('Priority') as HTMLSelectElement).value).toBe('elevated');
    expect((screen.getByLabelText('Section') as HTMLSelectElement).value).toBe('backlog');
    expect(screen.getByText('one')).toBeDefined();
    expect(screen.getByText('two')).toBeDefined();
    expect(screen.getByText('abcd')).toBeDefined();
    expect(screen.getByText('A comment')).toBeDefined();
    expect(screen.queryByLabelText('Title')).toBeNull();
    expect(screen.queryByLabelText('Description')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Save' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Edit title' })).toBeDefined();
    expect(screen.getByRole('button', { name: 'Edit description' })).toBeDefined();
    expect(screen.getByRole('button', { name: 'Edit tags' })).toBeDefined();
  });

  it('edits the title inline', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Edit title' }));
    expect((screen.getByLabelText('Title') as HTMLInputElement).value).toBe('Write tests');

    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'New title' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save title' }));

    await waitFor(() => {
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
    });
    await waitFor(() => {
      expect(screen.queryByLabelText('Title')).toBeNull();
    });
  });

  it('rejects an empty title', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Edit title' }));
    fireEvent.change(screen.getByLabelText('Title'), { target: { value: '   ' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save title' }));

    expect(await screen.findByText('A task needs a title.')).toBeDefined();
    expect(enqueue).not.toHaveBeenCalled();
  });

  it('cancels title editing without saving', () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Edit title' }));
    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'Changed title' } });
    fireEvent.click(screen.getByRole('button', { name: 'Cancel title edit' }));

    expect(enqueue).not.toHaveBeenCalled();
    expect(screen.queryByLabelText('Title')).toBeNull();
    expect(screen.getByRole('heading', { name: 'Write tests' })).toBeDefined();
  });

  it('edits the description inline', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Edit description' }));
    expect((screen.getByLabelText('Description') as HTMLTextAreaElement).value).toBe('Details');

    fireEvent.change(screen.getByLabelText('Description'), {
      target: { value: 'Updated details' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save description' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'update_task',
          id: TASK_ID,
          description: 'Updated details',
        }),
      );
    });
    await waitFor(() => {
      expect(screen.queryByLabelText('Description')).toBeNull();
    });
  });

  it('edits tags inline', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Edit tags' }));
    expect((screen.getByLabelText('Tags') as HTMLInputElement).value).toBe('one, two');

    fireEvent.change(screen.getByLabelText('Tags'), { target: { value: 'three, four' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save tags' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'update_task',
          id: TASK_ID,
          tags: ['four', 'three'],
        }),
      );
    });
  });

  it('changes priority inline', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.change(screen.getByLabelText('Priority'), { target: { value: 'normal' } });

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({ kind: 'update_task', id: TASK_ID, priority: 'normal' }),
      );
    });
  });

  it('moves the task when the section changes', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.change(screen.getByLabelText('Section'), { target: { value: 'doing' } });

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'move_task',
          id: TASK_ID,
          section_id: 'doing',
          position: 'a5',
        }),
      );
    });
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

  it('adds a comment', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.change(screen.getByLabelText('New comment'), {
      target: { value: '  A new comment  ' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Add comment' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'create_comment',
          task_id: TASK_ID,
          contents: 'A new comment',
        }),
      );
    });
  });

  it('edits a comment inline', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Edit comment' }));
    fireEvent.change(screen.getByLabelText('Comment contents'), {
      target: { value: 'Updated comment' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save comment' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'update_comment',
          id: 'comment-1',
          contents: 'Updated comment',
        }),
      );
    });
  });

  it('deletes a comment after two-step confirmation', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderTask(projectSnapshot(), enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Delete comment' }));
    fireEvent.click(screen.getByRole('button', { name: 'Confirm delete comment' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({ kind: 'delete_comment', id: 'comment-1' }),
      );
    });
  });

  it('locks synced fields and shows the GitHub link', () => {
    renderTask(
      projectSnapshot({
        tasks: [
          task({
            id: TASK_ID,
            title: 'Write tests',
            description: 'Details',
            section_id: 'backlog',
            external_identity: GITHUB_IDENTITY,
            external_metadata: metadata(),
          }),
        ],
      }),
      vi.fn(async () => undefined),
    );

    expect(screen.queryByRole('button', { name: 'Edit title' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Edit description' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Close task' })).toBeNull();
    expect(screen.getByRole('link', { name: 'acme/widgets#123' }).getAttribute('href')).toBe(
      'https://github.com/acme/widgets/issues/123',
    );
    expect(screen.getByText(/Last synced/)).toBeDefined();
  });

  it('shows the GitHub error badge', () => {
    renderTask(
      projectSnapshot({
        tasks: [
          task({
            id: TASK_ID,
            title: 'Write tests',
            section_id: 'backlog',
            external_identity: GITHUB_IDENTITY,
            external_metadata: metadata({
              title: null,
              description: null,
              state: null,
              error: 'not_found',
            }),
          }),
        ],
      }),
      vi.fn(async () => undefined),
    );

    expect(screen.getByText('GitHub: not_found')).toBeDefined();
  });
});
