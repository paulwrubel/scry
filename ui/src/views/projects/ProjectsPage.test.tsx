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
    expect(screen.getByTestId('path').textContent).toBe('/projects/project-2');
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
