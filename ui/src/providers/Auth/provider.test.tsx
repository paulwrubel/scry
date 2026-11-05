// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { useState } from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { notifyUnauthorized } from '@/utils/authEvents';
import { useAuth } from './hook';
import { AuthProvider } from './provider';

type Call = { url: string; init?: RequestInit };

function jsonOrEmpty(status: number): Response {
  return new Response(null, { status });
}

type FetcherBehavior = {
  probeStatus?: number;
  loginStatus?: (body: unknown) => number;
};

function createFetcher(behavior: FetcherBehavior = {}) {
  const calls: Call[] = [];

  const fetcher = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const url = String(input);
    calls.push({ url, init });
    const method = init?.method ?? 'GET';

    if (url.endsWith('/auth/session')) {
      if (method === 'GET') {
        return jsonOrEmpty(behavior.probeStatus ?? 204);
      }
      if (method === 'POST') {
        const rawBody = typeof init?.body === 'string' ? init.body : 'null';
        const status = behavior.loginStatus ? behavior.loginStatus(JSON.parse(rawBody)) : 204;
        return jsonOrEmpty(status);
      }
      if (method === 'DELETE') {
        return jsonOrEmpty(204);
      }
    }

    return jsonOrEmpty(404);
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

function Probe() {
  const { isAuthenticated, isAuthLoading, login, logout } = useAuth();
  const [message, setMessage] = useState('');

  async function handleLogin() {
    const result = await login('secret');
    setMessage(result.ok ? 'Signed in' : result.message);
  }

  return (
    <div>
      <span data-testid="authenticated">{isAuthenticated ? 'true' : 'false'}</span>
      <span data-testid="auth-loading">{isAuthLoading ? 'true' : 'false'}</span>
      <span data-testid="message">{message}</span>
      <button onClick={() => void handleLogin()}>login</button>
      <button onClick={() => void logout()}>logout</button>
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

afterEach(() => {
  cleanup();
  localStorage.clear();
  sessionStorage.clear();
});

describe('AuthProvider', () => {
  it('probe 204 authenticates', async () => {
    const { fetcher } = createFetcher({ probeStatus: 204 });
    renderAuth(fetcher);

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
      expect(screen.getByTestId('auth-loading').textContent).toBe('false');
    });
  });

  it('probe 401 shows login', async () => {
    const { fetcher } = createFetcher({ probeStatus: 401 });
    renderAuth(fetcher);

    await waitFor(() => {
      expect(screen.getByTestId('auth-loading').textContent).toBe('false');
      expect(screen.getByTestId('authenticated').textContent).toBe('false');
    });
  });

  it('login sends the token only in the request body', async () => {
    const { fetcher, calls } = createFetcher({ probeStatus: 204 });
    renderAuth(fetcher);

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
    });

    fireEvent.click(screen.getByRole('button', { name: 'login' }));

    await waitFor(() => {
      expect(screen.getByTestId('message').textContent).toBe('Signed in');
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
    });

    const post = calls.find((call) => call.init?.method === 'POST');
    expect(post).toBeDefined();
    expect(post?.url).toBe('http://test/api/v1/auth/session');
    expect(post?.init?.body).toBe(JSON.stringify({ token: 'secret' }));
    expect(storageContains(localStorage, 'secret')).toBe(false);
    expect(storageContains(sessionStorage, 'secret')).toBe(false);
  });

  it('wrong token reports an error', async () => {
    const { fetcher } = createFetcher({ probeStatus: 401, loginStatus: () => 401 });
    renderAuth(fetcher);

    await waitFor(() => {
      expect(screen.getByTestId('auth-loading').textContent).toBe('false');
      expect(screen.getByTestId('authenticated').textContent).toBe('false');
    });

    fireEvent.click(screen.getByRole('button', { name: 'login' }));

    await waitFor(() => {
      expect(screen.getByTestId('message').textContent).toBe('Invalid API token');
      expect(screen.getByTestId('authenticated').textContent).toBe('false');
    });
  });

  it('logout deletes the session', async () => {
    const { fetcher, calls } = createFetcher({ probeStatus: 204 });
    renderAuth(fetcher);

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
    });

    fireEvent.click(screen.getByRole('button', { name: 'logout' }));

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('false');
    });

    expect(calls.some((call) => call.init?.method === 'DELETE')).toBe(true);
  });

  it('logout broadcasts signed-out', async () => {
    const channel = new FakeChannel();
    const { fetcher } = createFetcher({ probeStatus: 204 });
    renderAuth(fetcher, channel);

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
    });

    fireEvent.click(screen.getByRole('button', { name: 'logout' }));

    await waitFor(() => {
      expect(channel.messages).toContainEqual({ type: 'signed-out' });
    });
  });

  it('receives signed-out from another tab', async () => {
    const channel = new FakeChannel();
    const { fetcher } = createFetcher({ probeStatus: 204 });
    renderAuth(fetcher, channel);

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
    });

    act(() => {
      channel.emit({ type: 'signed-out' });
    });

    expect(screen.getByTestId('authenticated').textContent).toBe('false');
  });

  it('receives signed-in from another tab', async () => {
    const channel = new FakeChannel();
    const { fetcher } = createFetcher({ probeStatus: 401 });
    renderAuth(fetcher, channel);

    await waitFor(() => {
      expect(screen.getByTestId('auth-loading').textContent).toBe('false');
      expect(screen.getByTestId('authenticated').textContent).toBe('false');
    });

    act(() => {
      channel.emit({ type: 'signed-in' });
    });

    expect(screen.getByTestId('authenticated').textContent).toBe('true');
  });

  it('unauthorized event signs out', async () => {
    const { fetcher } = createFetcher({ probeStatus: 204 });
    renderAuth(fetcher);

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('true');
    });

    act(() => {
      notifyUnauthorized();
    });

    await waitFor(() => {
      expect(screen.getByTestId('authenticated').textContent).toBe('false');
    });
  });
});
