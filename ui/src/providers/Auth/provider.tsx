import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createSession, deleteSession, getAPIURL, getSessionStatus } from '@/utils/api';
import { AUTH_CHANNEL_NAME, isAuthChannelMessage, subscribeUnauthorized } from '@/utils/authEvents';
import { AuthContext } from './context';
import type { AuthContextType, LoginResult } from './context';

export type AuthProviderProps = {
  children: React.ReactNode;
  fetcher?: typeof fetch;
  baseUrl?: string;
  /** Omit to create a BroadcastChannel; pass null to simulate unsupported. */
  channel?: BroadcastChannel | null;
};

export function AuthProvider(props: AuthProviderProps) {
  const { children, fetcher = fetch, baseUrl = getAPIURL(), channel } = props;

  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isAuthLoading, setIsAuthLoading] = useState(true);

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

  const notifyTabs = useCallback((type: 'signed-in' | 'signed-out') => {
    channelRef.current?.postMessage({ type });
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
      setIsAuthenticated(data.type === 'signed-in');
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
  }, [channel, hasChannel]);

  useEffect(() => {
    let cancelled = false;

    getSessionStatus(fetcher, baseUrl)
      .then((response) => {
        if (!cancelled) {
          setIsAuthenticated(response.ok);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setIsAuthenticated(false);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsAuthLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [baseUrl, fetcher]);

  useEffect(
    () =>
      subscribeUnauthorized(() => {
        setIsAuthenticated(false);
        setIsAuthLoading(false);
        notifyTabs('signed-out');
      }),
    [notifyTabs],
  );

  const login = useCallback(
    async (token: string): Promise<LoginResult> => {
      try {
        const response = await createSession(fetcher, baseUrl, token);
        if (response.ok) {
          setIsAuthenticated(true);
          setIsAuthLoading(false);
          notifyTabs('signed-in');
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
    [baseUrl, fetcher, notifyTabs],
  );

  const logout = useCallback(async () => {
    try {
      await deleteSession(fetcher, baseUrl);
    } catch {
      // clear local auth state even if the request fails
    }
    setIsAuthenticated(false);
    notifyTabs('signed-out');
  }, [baseUrl, fetcher, notifyTabs]);

  const value = useMemo<AuthContextType>(
    () => ({ isAuthenticated, isAuthLoading, login, logout }),
    [isAuthenticated, isAuthLoading, login, logout],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
