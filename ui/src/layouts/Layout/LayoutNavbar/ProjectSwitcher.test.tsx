// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MemoryRouter, useLocation } from 'react-router-dom';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { ActiveProjectContext } from '@/providers/ActiveProject/context';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import type { Replica } from '@/utils/replica';
import { ProjectSwitcher } from './ProjectSwitcher';

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
    automations: [],
  };
}

function LocationProbe() {
  const location = useLocation();
  return <span data-testid="path">{location.pathname}</span>;
}

function renderSwitcher(path: string, activeProjectId = 'p1') {
  const projects = [projectSnapshot('p1', 'Alpha'), projectSnapshot('p2', 'Beta')];
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
      enqueue: async () => undefined,
    } as unknown as Replica,
    error: undefined,
    retry: () => undefined,
  };

  const setActiveProjectId = vi.fn();

  render(
    <ReplicaContext.Provider value={replicaContext}>
      <ActiveProjectContext.Provider
        value={{
          activeProjectId,
          setActiveProjectId,
          clearActiveProject: () => undefined,
        }}
      >
        <MemoryRouter initialEntries={[path]}>
          <ProjectSwitcher />
          <LocationProbe />
        </MemoryRouter>
      </ActiveProjectContext.Provider>
    </ReplicaContext.Provider>,
  );

  return { setActiveProjectId };
}

afterEach(cleanup);

describe('ProjectSwitcher', () => {
  it('shows the active project name in the trigger', () => {
    renderSwitcher('/projects/p1/tasks');

    expect(screen.getByRole('button', { name: 'Alpha' })).toBeDefined();
  });

  it('switches projects on the tasks tab', async () => {
    const { setActiveProjectId } = renderSwitcher('/projects/p1/tasks');

    fireEvent.click(screen.getByRole('button', { name: 'Alpha' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Beta' }));

    expect(setActiveProjectId).toHaveBeenCalledWith('p2');
    expect(screen.getByTestId('path').textContent).toBe('/projects/p2/tasks');
  });

  it('keeps the settings tab when switching projects', async () => {
    const { setActiveProjectId } = renderSwitcher('/projects/p1/settings');

    fireEvent.click(screen.getByRole('button', { name: 'Alpha' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Beta' }));

    expect(setActiveProjectId).toHaveBeenCalledWith('p2');
    expect(screen.getByTestId('path').textContent).toBe('/projects/p2/settings');
  });

  it('opens the new project form from the menu', async () => {
    renderSwitcher('/projects/p1/tasks');

    fireEvent.click(screen.getByRole('button', { name: 'Alpha' }));
    fireEvent.click(await screen.findByRole('button', { name: 'New project' }));

    expect(await screen.findByLabelText('Name')).toBeDefined();
  });
});
