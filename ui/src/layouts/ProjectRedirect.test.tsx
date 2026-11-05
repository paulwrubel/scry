// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { ActiveProjectContext } from '@/providers/ActiveProject/context';
import { ProjectRedirect } from './ProjectRedirect';

afterEach(cleanup);

function renderWithActiveProject(activeProjectId: string | null) {
  const router = createMemoryRouter(
    [
      { path: '/', element: <ProjectRedirect /> },
      { path: '/projects', element: <div>projects</div> },
      { path: '/projects/:projectId', element: <div>board</div> },
    ],
    { initialEntries: ['/'] },
  );

  render(
    <ActiveProjectContext.Provider
      value={{
        activeProjectId,
        setActiveProjectId: () => undefined,
        clearActiveProject: () => undefined,
      }}
    >
      <RouterProvider router={router} />
    </ActiveProjectContext.Provider>,
  );
}

describe('ProjectRedirect', () => {
  it('redirects to the active project board', async () => {
    renderWithActiveProject('p1');

    expect(await screen.findByText('board')).toBeTruthy();
  });

  it('redirects to the projects list when no project is active', async () => {
    renderWithActiveProject(null);

    expect(await screen.findByText('projects')).toBeTruthy();
  });
});
