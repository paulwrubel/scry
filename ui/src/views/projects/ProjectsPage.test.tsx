// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { ActiveProjectContext } from '@/providers/ActiveProject/context';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import type { Replica } from '@/utils/replica';
import { ProjectsPage } from './index';

const CREATED_AT = '2026-01-01T00:00:00Z';

function projectSnapshot(id: string, name: string): ProjectSnapshot {
  return {
    id,
    name,
    description: null,
    task_sorting_mode: 'manual',
    show_priority: false,
    created_at: CREATED_AT,
    sections: [],
    tasks: [],
    notes: [],
  };
}

function LocationProbe() {
  const location = useLocation();
  return <span data-testid="path">{location.pathname}</span>;
}

function renderPage(
  projects: ProjectSnapshot[],
  activeProjectId: string | null,
  enqueue: (operation: Operation) => Promise<void>,
) {
  const snapshot = {
    confirmed: { projects },
    visible: { projects },
    outbox: [],
    needsBootstrap: false,
    persisted: true,
    lastSeenSeq: 0,
  };

  const replicaContext: ReplicaContextValue = {
    status: 'ready',
    replica: {
      getSnapshot: () => snapshot,
      subscribe: () => () => undefined,
      enqueue,
    } as unknown as Replica,
    error: undefined,
    retry: () => undefined,
  };

  const setActiveProjectId = vi.fn();
  const clearActiveProject = vi.fn();

  render(
    <ReplicaContext.Provider value={replicaContext}>
      <ActiveProjectContext.Provider
        value={{ activeProjectId, setActiveProjectId, clearActiveProject }}
      >
        <MemoryRouter initialEntries={['/projects']}>
          <LocationProbe />
          <ProjectsPage />
        </MemoryRouter>
      </ActiveProjectContext.Provider>
    </ReplicaContext.Provider>,
  );

  return { setActiveProjectId, clearActiveProject };
}

afterEach(cleanup);

describe('ProjectsPage deletion', () => {
  it('switches to the next project after deleting the active one', async () => {
    const enqueue = vi.fn(async () => undefined);
    const { setActiveProjectId, clearActiveProject } = renderPage(
      [projectSnapshot('project-1', 'Alpha'), projectSnapshot('project-2', 'Beta')],
      'project-1',
      enqueue,
    );

    fireEvent.click(screen.getAllByRole('button', { name: 'Delete' })[0]);
    fireEvent.click(screen.getByRole('button', { name: 'Delete project' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({ kind: 'delete_project', id: 'project-1' }),
      );
    });
    await waitFor(() => {
      expect(setActiveProjectId).toHaveBeenCalledWith('project-2');
    });
    expect(clearActiveProject).not.toHaveBeenCalled();
    await waitFor(() => {
      expect(screen.getByTestId('path').textContent).toBe('/projects/project-2/tasks');
    });
  });

  it('stays on the list when deleting a non-active project', async () => {
    const enqueue = vi.fn(async () => undefined);
    const { setActiveProjectId, clearActiveProject } = renderPage(
      [projectSnapshot('project-1', 'Alpha'), projectSnapshot('project-2', 'Beta')],
      'project-2',
      enqueue,
    );

    fireEvent.click(screen.getAllByRole('button', { name: 'Delete' })[0]);
    fireEvent.click(screen.getByRole('button', { name: 'Delete project' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({ kind: 'delete_project', id: 'project-1' }),
      );
    });
    expect(setActiveProjectId).not.toHaveBeenCalled();
    expect(clearActiveProject).not.toHaveBeenCalled();
    expect(screen.getByTestId('path').textContent).toBe('/projects');
  });

  it('clears the active project when deleting the last project', async () => {
    const enqueue = vi.fn(async () => undefined);
    const { setActiveProjectId, clearActiveProject } = renderPage(
      [projectSnapshot('project-1', 'Only')],
      'project-1',
      enqueue,
    );

    fireEvent.click(screen.getAllByRole('button', { name: 'Delete' })[0]);
    fireEvent.click(screen.getByRole('button', { name: 'Delete project' }));

    await waitFor(() => {
      expect(clearActiveProject).toHaveBeenCalledTimes(1);
    });
    expect(setActiveProjectId).not.toHaveBeenCalled();
    expect(screen.getByTestId('path').textContent).toBe('/projects');
  });
});

describe('ProjectsPage creation', () => {
  it('creates a project and opens its board', async () => {
    const enqueue = vi.fn(async (_operation: Operation) => undefined);
    const { setActiveProjectId, clearActiveProject } = renderPage([], null, enqueue);

    fireEvent.click(screen.getAllByRole('button', { name: 'New project' })[0]);
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'New project' } });
    fireEvent.change(screen.getByLabelText('Description'), { target: { value: 'Details' } });

    const create = screen.getByRole('button', { name: 'Create' }) as HTMLButtonElement;
    await waitFor(() => {
      expect(create.disabled).toBe(false);
    });
    fireEvent.click(create);

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledTimes(1);
    });

    const operation = enqueue.mock.calls[0][0];
    expect(operation).toMatchObject({
      kind: 'create_project',
      name: 'New project',
      description: 'Details',
      task_sorting_mode: 'manual',
      show_priority: false,
    });
    await waitFor(() => {
      expect(setActiveProjectId).toHaveBeenCalledWith(operation.id);
    });
    expect(clearActiveProject).not.toHaveBeenCalled();
    expect(screen.getByTestId('path').textContent).toBe(`/projects/${operation.id}/tasks`);
  });

  it('opens the create form from the empty state', () => {
    renderPage(
      [],
      null,
      vi.fn(async () => undefined),
    );

    const buttons = screen.getAllByRole('button', { name: 'New project' });
    expect(buttons).toHaveLength(2);

    fireEvent.click(buttons[1]);

    expect(screen.getByLabelText('Name')).toBeDefined();
  });
});
