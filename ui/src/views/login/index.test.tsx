// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { AuthContext } from '@/providers/Auth/context';
import type { AuthContextType, LoginResult } from '@/providers/Auth/context';
import { LoginPage } from './index';

afterEach(cleanup);

type RenderLoginOptions = {
  isAuthenticated?: boolean;
  login?: (password: string) => Promise<LoginResult>;
  initialEntries?: string[];
};

function renderLogin(options: RenderLoginOptions = {}) {
  const login = options.login ?? vi.fn(async (): Promise<LoginResult> => ({ ok: true }));
  const value: AuthContextType = {
    accessToken: options.isAuthenticated ? 'test-token' : undefined,
    isAuthenticated: options.isAuthenticated ?? false,
    isAuthLoading: false,
    authenticatedFetch: fetch,
    login,
    logout: async () => undefined,
  };

  render(
    <AuthContext.Provider value={value}>
      <MemoryRouter initialEntries={options.initialEntries ?? ['/login']}>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/" element={<div>home</div>} />
        </Routes>
      </MemoryRouter>
    </AuthContext.Provider>,
  );

  return { login };
}

describe('LoginPage', () => {
  it('renders the sign-in form', () => {
    renderLogin();

    expect(screen.getByRole('heading', { name: 'Sign in' })).toBeTruthy();
    expect(screen.getByLabelText('Please enter the password')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Sign in' })).toBeTruthy();
  });

  it('toggles password visibility', () => {
    renderLogin();

    const input = screen.getByLabelText('Please enter the password');
    expect(input.getAttribute('type')).toBe('password');

    fireEvent.click(screen.getByRole('button', { name: 'Show password' }));
    expect(input.getAttribute('type')).toBe('text');

    fireEvent.click(screen.getByRole('button', { name: 'Hide password' }));
    expect(input.getAttribute('type')).toBe('password');
  });

  it('submits the password', async () => {
    const { login } = renderLogin();

    fireEvent.change(screen.getByLabelText('Please enter the password'), {
      target: { value: 'secret' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Sign in' }));

    await waitFor(() => {
      expect(login).toHaveBeenCalledWith('secret');
    });
  });

  it('shows the login error', async () => {
    renderLogin({ login: async () => ({ ok: false, message: 'Invalid admin password' }) });

    fireEvent.change(screen.getByLabelText('Please enter the password'), {
      target: { value: 'wrong' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Sign in' }));

    expect(await screen.findByText('Invalid admin password')).toBeTruthy();
  });

  it('redirects when already authenticated', async () => {
    renderLogin({ isAuthenticated: true });

    expect(await screen.findByText('home')).toBeTruthy();
  });
});
