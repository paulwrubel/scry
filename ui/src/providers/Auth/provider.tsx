import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { bootstrapAuth, cacheBootstrapSession, resetBootstrapCache } from '@/layouts/authLoader';
import { createSession, getAPIURL, logoutSession, refreshSession } from '@/utils/api';
import type { AccessTokenResponse } from '@/utils/api';
import { AUTH_CHANNEL_NAME, isAuthChannelMessage } from '@/utils/authEvents';
import type { AuthChannelMessage } from '@/utils/authEvents';
import { AuthContext } from './context';
import type { AuthContextType, LoginResult } from './context';

export type AuthProviderProps = {
  children: React.ReactNode;
  fetcher?: typeof fetch;
  baseUrl?: string;
  /** Omit to create a BroadcastChannel; pass null to simulate unsupported. */
  channel?: BroadcastChannel | null;
};

type RefreshResult = string | null;

export function AuthProvider(props: AuthProviderProps) {
  const { children, fetcher = fetch, baseUrl = getAPIURL(), channel } = props;

  const [accessToken, setAccessToken] = useState<string | undefined>(undefined);
  const [isAuthLoading, setIsAuthLoading] = useState(true);

  // authenticatedFetch reads the token through a ref so its identity stays
  // stable across token changes; state mirrors the ref for consumers.
  const accessTokenRef = useRef<string | undefined>(undefined);
  const refreshPromiseRef = useRef<Promise<RefreshResult> | null>(null);

  const applyAccessToken = useCallback((token: string | undefined) => {
    accessTokenRef.current = token;
    setAccessToken(token);
  }, []);

  // Auth state is shared across tabs: a login or logout in one tab should not
  // leave another tab showing a stale screen. The channel is created in an
  // effect so StrictMode's setup/cleanup cycle leaves a working one.
  const [hasChannel] = useState(() => {
    if (channel !== undefined) {
      return channel !== null;
    }
    return typeof BroadcastChannel !== 'undefined';
  });
  const channelRef = useRef<BroadcastChannel | null>(null);

  const notifyTabs = useCallback((message: AuthChannelMessage) => {
    channelRef.current?.postMessage(message);
  }, []);

  useEffect(() => {
    if (!hasChannel) {
      return;
    }

    const active = channel !== undefined ? channel : new BroadcastChannel(AUTH_CHANNEL_NAME);
    if (active === null) {
      return;
    }
    channelRef.current = active;

    function handleMessage(event: MessageEvent) {
      const data = event.data as unknown;
      if (!isAuthChannelMessage(data)) {
        return;
      }
      if (data.type === 'signed-in') {
        applyAccessToken(data.accessToken);
        cacheBootstrapSession(data.accessToken);
      } else {
        applyAccessToken(undefined);
        resetBootstrapCache();
      }
      setIsAuthLoading(false);
    }

    active.addEventListener('message', handleMessage);
    return () => {
      active.removeEventListener('message', handleMessage);
      if (channelRef.current === active) {
        channelRef.current = null;
      }
      if (channel === undefined) {
        active.close();
      }
    };
  }, [applyAccessToken, channel, hasChannel]);

  const refreshAccessToken = useCallback(async (): Promise<RefreshResult> => {
    const inFlight = refreshPromiseRef.current;
    if (inFlight !== null) {
      return inFlight;
    }

    const request = (async () => {
      try {
        const response = await refreshSession(fetcher, baseUrl);
        if (!response.ok) {
          resetBootstrapCache();
          return null;
        }
        const body = (await response.json()) as AccessTokenResponse;
        if (typeof body.access_token !== 'string' || body.access_token === '') {
          resetBootstrapCache();
          return null;
        }
        applyAccessToken(body.access_token);
        notifyTabs({ type: 'signed-in', accessToken: body.access_token });
        cacheBootstrapSession(body.access_token);
        return body.access_token;
      } catch {
        resetBootstrapCache();
        return null;
      } finally {
        refreshPromiseRef.current = null;
      }
    })();

    refreshPromiseRef.current = request;
    return request;
  }, [applyAccessToken, baseUrl, fetcher, notifyTabs]);

  useEffect(() => {
    let cancelled = false;

    bootstrapAuth(fetcher, baseUrl).then((session) => {
      if (cancelled) {
        return;
      }
      if (session !== null) {
        applyAccessToken(session.accessToken);
        notifyTabs({ type: 'signed-in', accessToken: session.accessToken });
      }
      setIsAuthLoading(false);
    });

    return () => {
      cancelled = true;
    };
  }, [applyAccessToken, baseUrl, fetcher, notifyTabs]);

  const login = useCallback(
    async (token: string): Promise<LoginResult> => {
      try {
        const response = await createSession(fetcher, baseUrl, token);
        if (response.ok) {
          let body: AccessTokenResponse;
          try {
            body = (await response.json()) as AccessTokenResponse;
          } catch {
            return { ok: false, message: 'Unable to sign in' };
          }
          if (typeof body.access_token !== 'string' || body.access_token === '') {
            return { ok: false, message: 'Unable to sign in' };
          }
          applyAccessToken(body.access_token);
          setIsAuthLoading(false);
          notifyTabs({ type: 'signed-in', accessToken: body.access_token });
          cacheBootstrapSession(body.access_token);
          return { ok: true };
        }
        if (response.status === 401) {
          return { ok: false, message: 'Invalid API token' };
        }
        if (response.status === 400) {
          return { ok: false, message: 'Authentication is disabled on this server' };
        }
        return { ok: false, message: 'Unable to sign in' };
      } catch {
        return { ok: false, message: 'Unable to reach the server' };
      }
    },
    [applyAccessToken, baseUrl, fetcher, notifyTabs],
  );

  const logout = useCallback(async () => {
    try {
      await logoutSession(fetcher, baseUrl);
    } catch {
      // clear local auth state even if the request fails
    }
    applyAccessToken(undefined);
    notifyTabs({ type: 'signed-out' });
    resetBootstrapCache();
    setIsAuthLoading(false);
  }, [applyAccessToken, baseUrl, fetcher, notifyTabs]);

  const authenticatedFetch = useCallback<typeof fetch>(
    async (input, init) => {
      function withAuthorization(token: string | undefined): RequestInit {
        const headers = new Headers(init?.headers);
        if (token !== undefined) {
          headers.set('Authorization', `Bearer ${token}`);
        }
        return { ...init, headers };
      }

      const response = await fetcher(input, withAuthorization(accessTokenRef.current));
      if (response.status !== 401) {
        return response;
      }

      const refreshed = await refreshAccessToken();
      if (refreshed === null) {
        // an automatic refresh failure clears the token but must not sign
        // other tabs out, since the refresh cookie may still be usable.
        applyAccessToken(undefined);
        return response;
      }

      return fetcher(input, withAuthorization(refreshed));
    },
    [applyAccessToken, fetcher, refreshAccessToken],
  );

  const isAuthenticated = accessToken !== undefined;
  const value = useMemo<AuthContextType>(
    () => ({ accessToken, isAuthenticated, isAuthLoading, authenticatedFetch, login, logout }),
    [accessToken, isAuthenticated, isAuthLoading, authenticatedFetch, login, logout],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
