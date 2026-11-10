// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import type { Replica } from '@/utils/replica';
import { ProjectLayout } from './ProjectLayout';

const PROJECT_ID = 'project-1';
const CREATED_AT = '2026-01-01T00:00:00Z';

function projectSnapshot(): ProjectSnapshot {
  return {
    id: PROJECT_ID,
    name: 'Project',
    description: null,
    task_sorting_mode: 'manual',
    show_priority: false,
    created_at: CREATED_AT,
    sections: [],
    tasks: [
      {
        id: 'task-1',
        project_id: PROJECT_ID,
        title: 'Task title',
        description: null,
        priority: 'medium',
        section_id: null,
        position: 'a0',
        state: 'open',
        tags: [],
        created_at: CREATED_AT,
      },
    ],
    notes: [],
    automations: [],
  };
}

function LocationProbe() {
  const location = useLocation();
  return (
    <span data-testid="path">
      {location.pathname}
      {location.search}
    </span>
  );
}

function renderLayout(path: string) {
  const project = projectSnapshot();
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
      enqueue: async () => undefined,
    } as unknown as Replica,
    error: undefined,
    retry: () => undefined,
  };

  render(
    <ReplicaContext.Provider value={context}>
      <MemoryRouter initialEntries={[path]}>
        <LocationProbe />
        <Routes>
          <Route path="/projects/:projectId/tasks" element={<ProjectLayout />}>
            <Route path=":taskId" />
          </Route>
        </Routes>
      </MemoryRouter>
    </ReplicaContext.Provider>,
  );
}

afterEach(cleanup);

describe('ProjectLayout', () => {
  it('opens the task in a drawer over the board', () => {
    renderLayout(`/projects/${PROJECT_ID}/tasks/task-1`);

    expect(screen.getByRole('heading', { name: 'Task title' })).toBeDefined();
    expect(screen.getByTestId('flowbite-drawer').getAttribute('aria-hidden')).toBe('false');
  });

  it('closes the drawer on Escape', async () => {
    renderLayout(`/projects/${PROJECT_ID}/tasks/task-1`);

    fireEvent.keyDown(document, { key: 'Escape' });

    await waitFor(() => {
      expect(screen.getByTestId('path').textContent).toBe(`/projects/${PROJECT_ID}/tasks`);
    });
  });

  it('keeps the drawer closed on the board route', () => {
    renderLayout(`/projects/${PROJECT_ID}/tasks`);

    expect(screen.getByTestId('flowbite-drawer').getAttribute('aria-hidden')).toBe('true');
    expect(screen.queryByRole('heading', { name: 'Task title' })).toBeNull();
  });

  it('preserves the active filter when closing the drawer', async () => {
    renderLayout(`/projects/${PROJECT_ID}/tasks/task-1?state=open`);

    fireEvent.keyDown(document, { key: 'Escape' });

    await waitFor(() => {
      expect(screen.getByTestId('path').textContent).toBe(
        `/projects/${PROJECT_ID}/tasks?state=open`,
      );
    });
  });

  it('keeps the filter on the back-to-board link', () => {
    renderLayout(`/projects/${PROJECT_ID}/tasks/task-1?state=open`);

    expect(screen.getByRole('link', { name: 'Back to board' }).getAttribute('href')).toBe(
      `/projects/${PROJECT_ID}/tasks?state=open`,
    );
  });
});
