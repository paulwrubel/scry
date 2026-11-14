// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import { ProjectLayout } from './ProjectLayout';
import { createReplicaMock, makeProject, makeReplicaSnapshot, makeTask } from '@/test/fixtures';

const PROJECT_ID = 'project-1';

function projectSnapshot(): ProjectSnapshot {
  return makeProject({ tasks: [makeTask()] });
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
  const snapshot = makeReplicaSnapshot([project]);

  const context: ReplicaContextValue = {
    status: 'ready',
    replica: createReplicaMock(snapshot),
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

  it('shows the read-only task view when opening a task', () => {
    renderLayout(`/projects/${PROJECT_ID}/tasks/task-1`);

    expect(screen.queryByLabelText('Title')).toBeNull();
    expect(screen.getByRole('button', { name: 'Edit title' })).toBeDefined();
  });

  it('returns to view mode after closing and reopening the drawer', async () => {
    renderLayout(`/projects/${PROJECT_ID}/tasks/task-1`);

    fireEvent.click(screen.getByRole('button', { name: 'Edit title' }));
    expect(screen.getByLabelText('Title')).toBeDefined();

    fireEvent.keyDown(document, { key: 'Escape' });
    await waitFor(() => {
      expect(screen.getByTestId('path').textContent).toBe(`/projects/${PROJECT_ID}/tasks`);
    });

    fireEvent.click(screen.getByRole('link', { name: 'Task title' }));
    await waitFor(() => {
      expect(screen.getByTestId('path').textContent).toBe(`/projects/${PROJECT_ID}/tasks/task-1`);
    });

    expect(screen.queryByLabelText('Title')).toBeNull();
    expect(screen.getByRole('button', { name: 'Edit title' })).toBeDefined();
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
