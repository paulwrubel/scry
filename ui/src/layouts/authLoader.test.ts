// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { authLoader, bootstrapAuth, resetBootstrapCache } from './authLoader';

const BASE_URL = 'http://test/api/v1';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

beforeEach(() => {
  resetBootstrapCache();
});

afterEach(() => {
  resetBootstrapCache();
  vi.unstubAllGlobals();
});

describe('bootstrapAuth', () => {
  it('dedupes concurrent callers into one fetch', async () => {
    let resolveResponse: (response: Response) => void = () => undefined;
    const pending = new Promise<Response>((resolve) => {
      resolveResponse = resolve;
    });
    const fetcher = vi.fn(() => pending);

    const first = bootstrapAuth(fetcher as typeof fetch, BASE_URL);
    const second = bootstrapAuth(fetcher as typeof fetch, BASE_URL);

    expect(fetcher).toHaveBeenCalledTimes(1);
    resolveResponse(jsonResponse({ access_token: 'shared-token' }));

    await expect(first).resolves.toEqual({ accessToken: 'shared-token' });
    await expect(second).resolves.toEqual({ accessToken: 'shared-token' });
    expect(fetcher).toHaveBeenCalledTimes(1);
  });

  it('returns the session on a successful refresh', async () => {
    const fetcher = vi.fn(async () => jsonResponse({ access_token: 'token' }));

    const session = await bootstrapAuth(fetcher as typeof fetch, BASE_URL);
    expect(session).toEqual({ accessToken: 'token' });
  });

  it('returns null on failure and retries after the cache is reset', async () => {
    const failing = vi.fn(async () => jsonResponse({}, 401));
    const failed = await bootstrapAuth(failing as typeof fetch, BASE_URL);
    expect(failed).toBeNull();

    const succeeding = vi.fn(async () => jsonResponse({ access_token: 'token' }));
    const stillFailed = await bootstrapAuth(succeeding as typeof fetch, BASE_URL);
    expect(stillFailed).toBeNull();
    expect(succeeding).not.toHaveBeenCalled();

    resetBootstrapCache();

    const session = await bootstrapAuth(succeeding as typeof fetch, BASE_URL);
    expect(session).toEqual({ accessToken: 'token' });
    expect(succeeding).toHaveBeenCalledTimes(1);
  });
});

describe('authLoader', () => {
  it('returns the session when the bootstrap succeeds', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => jsonResponse({ access_token: 'loader-token' })),
    );
    const request = new Request('http://localhost/projects/abc?tab=board');

    await expect(authLoader({ request })).resolves.toEqual({ accessToken: 'loader-token' });
  });

  it('throws a redirect with the encoded path and query when the bootstrap fails', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => jsonResponse({}, 401)),
    );
    const request = new Request('http://localhost/projects/abc?tab=board');

    let thrown: unknown;
    try {
      await authLoader({ request });
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(Response);
    const response = thrown as Response;
    expect(response.status).toBe(302);
    expect(response.headers.get('Location')).toBe(
      '/login?redirect=%2Fprojects%2Fabc%3Ftab%3Dboard',
    );
  });
});
