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
    show_priority: false,
    created_at: CREATED_AT,
    sections: [],
    tasks: [],
    notes: [],
    automations: [],
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

describe('ProjectsPage', () => {
  it('redirects to the active project board when one is active', async () => {
    renderPage(
      [projectSnapshot('project-1', 'Alpha')],
      'project-1',
      vi.fn(async () => undefined),
    );

    await waitFor(() => {
      expect(screen.getByTestId('path').textContent).toBe('/projects/project-1/tasks');
    });
  });

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
