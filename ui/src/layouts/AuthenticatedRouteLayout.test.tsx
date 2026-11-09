// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { AuthContext } from '@/providers/Auth/context';
import type { AuthContextType } from '@/providers/Auth/context';
import { AuthenticatedRouteLayout } from './AuthenticatedRouteLayout';

afterEach(cleanup);

type RenderGuardOptions = {
  isAuthLoading?: boolean;
  isAuthenticated?: boolean;
};

function renderGuard(options: RenderGuardOptions = {}) {
  const value: AuthContextType = {
    accessToken: options.isAuthenticated ? 'test-token' : undefined,
    isAuthenticated: options.isAuthenticated ?? false,
    isAuthLoading: options.isAuthLoading ?? false,
    authenticatedFetch: fetch,
    login: async () => ({ ok: true }),
    logout: async () => undefined,
  };

  const router = createMemoryRouter(
    [
      {
        element: <AuthenticatedRouteLayout />,
        children: [{ path: 'projects', element: <div>outlet</div> }],
      },
      { path: '/login', element: <div>login</div> },
    ],
    { initialEntries: ['/projects'] },
  );

  render(
    <AuthContext.Provider value={value}>
      <RouterProvider router={router} />
    </AuthContext.Provider>,
  );

  return { router };
}

describe('AuthenticatedRouteLayout', () => {
  it('renders nothing before auth resolves', () => {
    renderGuard({ isAuthLoading: true });

    expect(screen.queryByText('outlet')).toBeNull();
    expect(screen.queryByText('login')).toBeNull();
  });

  it('redirects unauthenticated users to login with the return path', async () => {
    const { router } = renderGuard({ isAuthenticated: false });

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/login');
      expect(router.state.location.search).toBe('?redirect=%2Fprojects');
    });
  });

  it('renders the child route when authenticated', () => {
    renderGuard({ isAuthenticated: true });

    expect(screen.getByText('outlet')).toBeTruthy();
  });
});
