// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { StrictMode, useState } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { resetBootstrapCache } from '@/layouts/authLoader';
import { useAuth } from './hook';
import { AuthProvider } from './provider';

type Call = { url: string; init?: RequestInit };

type FetcherBehavior = {
  refreshStatus?: number;
  refreshStatuses?: number[];
  refreshTokens?: string[];
  loginStatus?: number;
  loginBody?: unknown;
  loginThrows?: boolean;
  loginInvalidJson?: boolean;
  apiResponses?: number[];
  logoutThrows?: boolean;
};

const REFRESH_TOKEN = 'refresh-access-token';
const LOGIN_TOKEN = 'login-access-token';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function createFetcher(behavior: FetcherBehavior = {}) {
  const calls: Call[] = [];
  let refreshIndex = 0;
  let apiIndex = 0;

  const fetcher = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const url = String(input);
    calls.push({ url, init });
    const method = init?.method ?? 'GET';

    if (url.endsWith('/auth/session') && method === 'POST') {
      if (behavior.loginThrows) {
        throw new Error('network down');
      }
      if (behavior.loginInvalidJson) {
        return new Response('not json', { status: 200 });
      }
      const status = behavior.loginStatus ?? 200;
      if (status !== 200) {
        return new Response(null, { status });
      }
      return jsonResponse(behavior.loginBody ?? { access_token: LOGIN_TOKEN });
    }

    if (url.endsWith('/auth/refresh') && method === 'POST') {
      const index = refreshIndex;
      refreshIndex += 1;
      const statuses = behavior.refreshStatuses ?? [behavior.refreshStatus ?? 200];
      const status = statuses[Math.min(index, statuses.length - 1)];
      if (status !== 200) {
        return new Response(null, { status });
      }
      const tokens = behavior.refreshTokens ?? [REFRESH_TOKEN];
      return jsonResponse({ access_token: tokens[Math.min(index, tokens.length - 1)] });
    }

    if (url.endsWith('/auth/logout') && method === 'POST') {
      if (behavior.logoutThrows) {
        throw new Error('network down');
      }
      return new Response(null, { status: 204 });
    }

    if (url.includes('/protected')) {
      const status = behavior.apiResponses?.[apiIndex] ?? 200;
      apiIndex += 1;
      return new Response(null, { status });
    }

    return new Response(null, { status: 404 });
  };

  return { fetcher: fetcher as typeof fetch, calls };
}

class FakeChannel {
  readonly messages: unknown[] = [];
  private listeners = new Set<(event: MessageEvent) => void>();

  addEventListener(_type: 'message', listener: (event: MessageEvent) => void) {
    this.listeners.add(listener);
  }

  removeEventListener(_type: 'message', listener: (event: MessageEvent) => void) {
    this.listeners.delete(listener);
  }

  postMessage(data: unknown) {
    this.messages.push(data);
  }

  emit(data: unknown) {
    for (const listener of this.listeners) {
      listener({ data } as MessageEvent);
    }
  }

  close() {
    // no transport to close in the fake
  }
}

class ClosableChannel {
  readonly messages: unknown[] = [];
  closed = false;
  private listeners = new Set<(event: MessageEvent) => void>();

  addEventListener(_type: 'message', listener: (event: MessageEvent) => void) {
    this.listeners.add(listener);
  }

  removeEventListener(_type: 'message', listener: (event: MessageEvent) => void) {
    this.listeners.delete(listener);
  }

  postMessage(data: unknown) {
    if (this.closed) {
      throw new DOMException('the channel is closed', 'InvalidStateError');
    }
    this.messages.push(data);
  }

  close() {
    this.closed = true;
  }
}

function Probe() {
  const { accessToken, isAuthenticated, isAuthLoading, authenticatedFetch, login, logout } =
    useAuth();
  const [message, setMessage] = useState('');
  const [status, setStatus] = useState('');

  async function handleLogin() {
    const result = await login('secret');
    setMessage(result.ok ? 'Signed in' : result.message);
  }

  async function handleFetch() {
    const response = await authenticatedFetch('/api/protected');
    setStatus(String(response.status));
  }

  return (
    <div>
      <span data-testid="authenticated">{isAuthenticated ? 'true' : 'false'}</span>
      <span data-testid="auth-loading">{isAuthLoading ? 'true' : 'false'}</span>
      <span data-testid="token">{accessToken ?? ''}</span>
      <span data-testid="message">{message}</span>
      <span data-testid="status">{status}</span>
      <button onClick={() => void handleLogin()}>login</button>
      <button onClick={() => void logout()}>logout</button>
      <button onClick={() => void handleFetch()}>fetch</button>
    </div>
  );
}

function renderAuth(fetcher: typeof fetch, channel?: FakeChannel) {
  return render(
    <AuthProvider
      fetcher={fetcher}
      baseUrl="http://test/api/v1"
      channel={channel === undefined ? null : (channel as unknown as BroadcastChannel)}
    >
      <Probe />
    </AuthProvider>,
  );
}

function storageContains(storage: Storage, needle: string): boolean {
  for (let index = 0; index < storage.length; index += 1) {
    const key = storage.key(index);
    if (key === null) {
      continue;
    }
    if (key.includes(needle) || (storage.getItem(key) ?? '').includes(needle)) {
      return true;
    }
  }
  return false;
}

async function expectSignedOut() {
  await waitFor(() => {
    expect(screen.getByTestId('auth-loading').textContent).toBe('false');
    expect(screen.getByTestId('authenticated').textContent).toBe('false');
  });
}

beforeEach(() => {
  resetBootstrapCache();
});

afterEach(() => {
  cleanup();
  localStorage.clear();
  sessionStorage.clear();
  resetBootstrapCache();
  vi.unstubAllGlobals();
});

describe('AuthProvider', () => {
  it('recovers the session with a refresh on mount', async () => {
    const { fetcher, calls } = createFetcher();
    renderAuth(fetcher);

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
      expect(screen.getByTestId('auth-loading').textContent).toBe('false');
    });

    const refresh = calls.find((call) => call.url.endsWith('/auth/refresh'));
    expect(refresh).toBeDefined();
    expect(refresh?.init?.method).toBe('POST');
    expect(refresh?.init?.credentials).toBe('include');
    expect(screen.getByTestId('token').textContent).toBe(REFRESH_TOKEN);
  });

  it('shows signed out when the initial refresh fails', async () => {
    const { fetcher } = createFetcher({ refreshStatus: 401 });
    renderAuth(fetcher);

    await expectSignedOut();
    expect(screen.getByTestId('token').textContent).toBe('');
  });

  it('login stores the access token in memory only', async () => {
    const { fetcher, calls } = createFetcher({ refreshStatus: 401 });
    renderAuth(fetcher);

    await expectSignedOut();

    fireEvent.click(screen.getByRole('button', { name: 'login' }));

    await waitFor(() => {
      expect(screen.getByTestId('message').textContent).toBe('Signed in');
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
      expect(screen.getByTestId('token').textContent).toBe(LOGIN_TOKEN);
    });

    const post = calls.find((call) => call.url.endsWith('/auth/session'));
    expect(post).toBeDefined();
    expect(post?.url).toBe('http://test/api/v1/auth/session');
    expect(post?.init?.body).toBe(JSON.stringify({ password: 'secret' }));
    expect(post?.init?.credentials).toBe('include');
    expect(storageContains(localStorage, 'secret')).toBe(false);
    expect(storageContains(sessionStorage, 'secret')).toBe(false);
    expect(storageContains(localStorage, LOGIN_TOKEN)).toBe(false);
    expect(storageContains(sessionStorage, LOGIN_TOKEN)).toBe(false);
  });

  it('reports an invalid password', async () => {
    const { fetcher } = createFetcher({ refreshStatus: 401, loginStatus: 401 });
    renderAuth(fetcher);

    await expectSignedOut();
    fireEvent.click(screen.getByRole('button', { name: 'login' }));

    await waitFor(() => {
      expect(screen.getByTestId('message').textContent).toBe('Invalid password');
      expect(screen.getByTestId('authenticated').textContent).toBe('false');
    });
  });

  it('reports disabled authentication', async () => {
    const { fetcher } = createFetcher({ refreshStatus: 401, loginStatus: 400 });
    renderAuth(fetcher);

    await expectSignedOut();
    fireEvent.click(screen.getByRole('button', { name: 'login' }));

    await waitFor(() => {
      expect(screen.getByTestId('message').textContent).toBe(
        'Authentication is disabled on this server',
      );
    });
  });

  it('reports an unreachable server', async () => {
    const { fetcher } = createFetcher({ refreshStatus: 401, loginThrows: true });
    renderAuth(fetcher);

    await expectSignedOut();
    fireEvent.click(screen.getByRole('button', { name: 'login' }));

    await waitFor(() => {
      expect(screen.getByTestId('message').textContent).toBe('Unable to reach the server');
    });
  });

  it('reports invalid JSON from the login response', async () => {
    const { fetcher } = createFetcher({ refreshStatus: 401, loginInvalidJson: true });
    renderAuth(fetcher);

    await expectSignedOut();
    fireEvent.click(screen.getByRole('button', { name: 'login' }));

    await waitFor(() => {
      expect(screen.getByTestId('message').textContent).toBe('Unable to sign in');
    });
  });

  it('adds the bearer token to authenticated requests', async () => {
    const { fetcher, calls } = createFetcher();
    renderAuth(fetcher);

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
    });

    fireEvent.click(screen.getByRole('button', { name: 'fetch' }));

    await waitFor(() => {
      expect(screen.getByTestId('status').textContent).toBe('200');
    });

    const call = calls.find((entry) => entry.url.includes('/protected'));
    expect(call).toBeDefined();
    const headers = new Headers(call?.init?.headers);
    expect(headers.get('Authorization')).toBe(`Bearer ${REFRESH_TOKEN}`);
    expect(calls.filter((entry) => entry.url.endsWith('/auth/refresh'))).toHaveLength(1);
  });

  it('refreshes once and retries after a 401', async () => {
    const channel = new FakeChannel();
    const { fetcher, calls } = createFetcher({
      refreshTokens: ['initial-token', 'retry-token'],
      apiResponses: [401, 200],
    });
    renderAuth(fetcher, channel);

    await waitFor(() => {
      expect(screen.getByTestId('token').textContent).toBe('initial-token');
    });

    fireEvent.click(screen.getByRole('button', { name: 'fetch' }));

    await waitFor(() => {
      expect(screen.getByTestId('status').textContent).toBe('200');
    });

    const apiCalls = calls.filter((entry) => entry.url.includes('/protected'));
    expect(apiCalls).toHaveLength(2);
    expect(new Headers(apiCalls[0].init?.headers).get('Authorization')).toBe(
      'Bearer initial-token',
    );
    expect(new Headers(apiCalls[1].init?.headers).get('Authorization')).toBe('Bearer retry-token');
    expect(screen.getByTestId('token').textContent).toBe('retry-token');
    expect(screen.getByTestId('authenticated').textContent).toBe('true');
    expect(channel.messages).toContainEqual({ type: 'signed-in', accessToken: 'retry-token' });
  });

  it('clears the token when an automatic refresh fails without signing out tabs', async () => {
    const channel = new FakeChannel();
    const { fetcher, calls } = createFetcher({
      refreshStatuses: [200, 401],
      refreshTokens: ['initial-token'],
      apiResponses: [401, 200],
    });
    renderAuth(fetcher, channel);

    await waitFor(() => {
      expect(screen.getByTestId('token').textContent).toBe('initial-token');
    });

    fireEvent.click(screen.getByRole('button', { name: 'fetch' }));

    await waitFor(() => {
      expect(screen.getByTestId('status').textContent).toBe('401');
    });

    expect(screen.getByTestId('authenticated').textContent).toBe('false');
    expect(screen.getByTestId('token').textContent).toBe('');
    expect(calls.filter((entry) => entry.url.includes('/protected'))).toHaveLength(1);
    expect(channel.messages).not.toContainEqual({ type: 'signed-out' });
  });

  it('logout calls the endpoint and broadcasts signed-out', async () => {
    const channel = new FakeChannel();
    const { fetcher, calls } = createFetcher();
    renderAuth(fetcher, channel);

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
    });

    fireEvent.click(screen.getByRole('button', { name: 'logout' }));

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('false');
    });

    const logoutCall = calls.find((call) => call.url.endsWith('/auth/logout'));
    expect(logoutCall).toBeDefined();
    expect(logoutCall?.init?.method).toBe('POST');
    expect(logoutCall?.init?.credentials).toBe('include');
    expect(screen.getByTestId('token').textContent).toBe('');
    expect(channel.messages).toContainEqual({ type: 'signed-out' });
  });

  it('logout clears local auth even when the request fails', async () => {
    const channel = new FakeChannel();
    const { fetcher } = createFetcher({ logoutThrows: true });
    renderAuth(fetcher, channel);

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
    });

    fireEvent.click(screen.getByRole('button', { name: 'logout' }));

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('false');
      expect(screen.getByTestId('auth-loading').textContent).toBe('false');
    });

    expect(screen.getByTestId('token').textContent).toBe('');
    expect(channel.messages).toContainEqual({ type: 'signed-out' });
  });

  it('accepts a signed-in broadcast with a token', async () => {
    const channel = new FakeChannel();
    const { fetcher } = createFetcher({ refreshStatus: 401 });
    renderAuth(fetcher, channel);

    await expectSignedOut();

    act(() => {
      channel.emit({ type: 'signed-in', accessToken: 'broadcast-token' });
    });

    expect(screen.getByTestId('authenticated').textContent).toBe('true');
    expect(screen.getByTestId('auth-loading').textContent).toBe('false');
    expect(screen.getByTestId('token').textContent).toBe('broadcast-token');
  });

  it('ignores a signed-in broadcast without a token', async () => {
    const channel = new FakeChannel();
    const { fetcher } = createFetcher({ refreshStatus: 401 });
    renderAuth(fetcher, channel);

    await expectSignedOut();

    act(() => {
      channel.emit({ type: 'signed-in' });
    });

    expect(screen.getByTestId('authenticated').textContent).toBe('false');
    expect(screen.getByTestId('token').textContent).toBe('');
  });

  it('accepts a signed-out broadcast', async () => {
    const channel = new FakeChannel();
    const { fetcher } = createFetcher();
    renderAuth(fetcher, channel);

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
    });

    act(() => {
      channel.emit({ type: 'signed-out' });
    });

    expect(screen.getByTestId('authenticated').textContent).toBe('false');
    expect(screen.getByTestId('token').textContent).toBe('');
  });

  it('keeps a working channel through the StrictMode cleanup', async () => {
    const channels: ClosableChannel[] = [];
    vi.stubGlobal(
      'BroadcastChannel',
      class {
        constructor() {
          const channel = new ClosableChannel();
          channels.push(channel);
          return channel;
        }
      },
    );

    const { fetcher } = createFetcher();

    render(
      <StrictMode>
        <AuthProvider fetcher={fetcher} baseUrl="http://test/api/v1">
          <Probe />
        </AuthProvider>
      </StrictMode>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
    });

    expect(channels.length).toBeGreaterThan(1);
    expect(channels[0].closed).toBe(true);
    expect(channels[channels.length - 1].messages).toContainEqual({
      type: 'signed-in',
      accessToken: REFRESH_TOKEN,
    });
  });
});
