// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { ActiveProjectContext } from '@/providers/ActiveProject/context';
import { LayoutBottomNav } from './index';

afterEach(cleanup);

function renderNav(path: string, activeProjectId: string | null) {
  const router = createMemoryRouter([{ path: '*', element: <LayoutBottomNav /> }], {
    initialEntries: [path],
  });

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

function link(name: string) {
  return screen.getByRole('link', { name });
}

describe('LayoutBottomNav', () => {
  it('renders the project-scoped tabs', () => {
    renderNav('/projects/p1/tasks', 'p1');

    expect(link('Tasks').getAttribute('href')).toBe('/projects/p1/tasks');
    expect(link('Settings').getAttribute('href')).toBe('/projects/p1/settings');
  });

  it('marks Tasks as the current page on the board', () => {
    renderNav('/projects/p1/tasks', 'p1');

    expect(link('Tasks').getAttribute('aria-current')).toBe('page');
  });

  it('keeps Tasks current on a task route', () => {
    renderNav('/projects/p1/tasks/t1', 'p1');

    expect(link('Tasks').getAttribute('aria-current')).toBe('page');
  });

  it('marks Settings as the current page', () => {
    renderNav('/projects/p1/settings', 'p1');

    expect(link('Settings').getAttribute('aria-current')).toBe('page');
    expect(link('Tasks').getAttribute('aria-current')).toBeNull();
  });

  it('renders nothing when no project is active', () => {
    renderNav('/projects', null);

    expect(screen.queryByRole('navigation', { name: 'Primary' })).toBeNull();
  });
});
