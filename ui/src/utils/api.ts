import type { SyncRequest } from '@/generated/SyncRequest';

export type AccessTokenResponse = { access_token: string };

export function getAPIURL(): string {
  // the UI and API share an origin in production (Rust embeds both), and Vite
  // proxies /api -> the dev server, so no port substitution is needed
  return `${window.location.origin}/api/v1`;
}

export async function postSync(
  fetcher: typeof fetch,
  baseUrl: string,
  request: SyncRequest,
): Promise<Response> {
  return fetcher(`${baseUrl}/sync`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });
}

export async function getSnapshot(fetcher: typeof fetch, baseUrl: string): Promise<Response> {
  return fetcher(`${baseUrl}/snapshot`);
}

export async function createSession(
  fetcher: typeof fetch,
  baseUrl: string,
  password: string,
): Promise<Response> {
  return fetcher(`${baseUrl}/auth/session`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ password }),
    credentials: 'include',
  });
}

export async function refreshSession(fetcher: typeof fetch, baseUrl: string): Promise<Response> {
  return fetcher(`${baseUrl}/auth/refresh`, {
    method: 'POST',
    credentials: 'include',
  });
}

export async function logoutSession(fetcher: typeof fetch, baseUrl: string): Promise<Response> {
  return fetcher(`${baseUrl}/auth/logout`, {
    method: 'POST',
    credentials: 'include',
  });
}
