import type { SyncRequest } from '@/generated/SyncRequest';
import { notifyUnauthorized } from './authEvents';

export function getAPIURL(): string {
  // the UI and API share an origin in production (Rust embeds both), and Vite
  // proxies /api -> the dev server, so no port substitution is needed
  return `${window.location.origin}/api/v1`;
}

function notifyIfUnauthorized(response: Response): Response {
  if (response.status === 401) {
    notifyUnauthorized();
  }
  return response;
}

export async function postSync(
  fetcher: typeof fetch,
  baseUrl: string,
  request: SyncRequest,
): Promise<Response> {
  return notifyIfUnauthorized(
    await fetcher(`${baseUrl}/sync`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
      credentials: 'include',
    }),
  );
}

export async function getSnapshot(fetcher: typeof fetch, baseUrl: string): Promise<Response> {
  return notifyIfUnauthorized(await fetcher(`${baseUrl}/snapshot`, { credentials: 'include' }));
}

export async function createSession(
  fetcher: typeof fetch,
  baseUrl: string,
  token: string,
): Promise<Response> {
  return fetcher(`${baseUrl}/auth/session`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ token }),
    credentials: 'include',
  });
}

export async function deleteSession(fetcher: typeof fetch, baseUrl: string): Promise<Response> {
  return fetcher(`${baseUrl}/auth/session`, {
    method: 'DELETE',
    credentials: 'include',
  });
}

export async function getSessionStatus(fetcher: typeof fetch, baseUrl: string): Promise<Response> {
  return fetcher(`${baseUrl}/auth/session`, { credentials: 'include' });
}
