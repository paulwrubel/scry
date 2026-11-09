import { redirect } from 'react-router-dom';
import { getAPIURL, refreshSession } from '@/utils/api';
import type { AccessTokenResponse } from '@/utils/api';

export type AuthSession = { accessToken: string };

let bootstrapPromise: Promise<AuthSession | null> | null = null;

export function bootstrapAuth(
  fetcher: typeof fetch = fetch,
  baseUrl: string = getAPIURL(),
): Promise<AuthSession | null> {
  if (bootstrapPromise !== null) {
    return bootstrapPromise;
  }

  bootstrapPromise = (async () => {
    try {
      const response = await refreshSession(fetcher, baseUrl);
      if (!response.ok) {
        return null;
      }
      const body = (await response.json()) as AccessTokenResponse;
      if (typeof body.access_token !== 'string' || body.access_token === '') {
        return null;
      }
      return { accessToken: body.access_token };
    } catch {
      return null;
    }
  })();

  return bootstrapPromise;
}

export function resetBootstrapCache(): void {
  bootstrapPromise = null;
}

export function cacheBootstrapSession(accessToken: string): void {
  bootstrapPromise = Promise.resolve({ accessToken });
}

export async function authLoader({ request }: { request: Request }): Promise<AuthSession> {
  const session = await bootstrapAuth();
  if (session !== null) {
    return session;
  }

  const { pathname, search } = new URL(request.url);
  throw redirect(`/login?redirect=${encodeURIComponent(pathname + search)}`);
}
