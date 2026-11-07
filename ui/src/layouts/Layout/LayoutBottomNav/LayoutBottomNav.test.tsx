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
  it('renders all tabs with the active project board destination', () => {
    renderNav('/projects/p1', 'p1');

    expect(link('View').getAttribute('href')).toBe('/projects/p1');
    expect(link('Projects').getAttribute('href')).toBe('/projects');
    expect(link('Stats').getAttribute('href')).toBe('/stats');
  });

  it('marks View as the current page on the board', () => {
    renderNav('/projects/p1', 'p1');

    expect(link('View').getAttribute('aria-current')).toBe('page');
  });

  it('keeps View current on a task route', () => {
    renderNav('/projects/p1/tasks/t1', 'p1');

    expect(link('View').getAttribute('aria-current')).toBe('page');
  });

  it('marks Projects as the current page on /projects and not View', () => {
    renderNav('/projects', 'p1');

    expect(link('Projects').getAttribute('aria-current')).toBe('page');
    expect(link('View').getAttribute('aria-current')).toBeNull();
  });

  it('points View at / when no project is active', () => {
    renderNav('/', null);

    expect(link('View').getAttribute('href')).toBe('/');
  });
});
