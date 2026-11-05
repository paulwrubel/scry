import { useCallback, useEffect, useMemo, useState } from 'react';
import { openReplica } from '@/utils/replica';
import type { Replica } from '@/utils/replica';
import { ReplicaContext } from './context';
import type { ReplicaContextValue, ReplicaStatus } from './context';

export type ReplicaProviderProps = {
  children: React.ReactNode;
  open?: () => Promise<Replica>;
};

export function ReplicaProvider(props: ReplicaProviderProps) {
  const { children, open = openReplica } = props;

  const [attempt, setAttempt] = useState(0);
  const [status, setStatus] = useState<ReplicaStatus>('loading');
  const [replica, setReplica] = useState<Replica | undefined>(undefined);
  const [error, setError] = useState<Error | undefined>(undefined);

  useEffect(() => {
    let cancelled = false;
    let opened: Replica | undefined;

    open()
      .then((next) => {
        opened = next;
        if (cancelled) {
          next.close();
          return;
        }
        setReplica(next);
        setStatus('ready');
      })
      .catch((cause: unknown) => {
        if (cancelled) {
          return;
        }
        setError(cause instanceof Error ? cause : new Error(String(cause)));
        setStatus('error');
      });

    return () => {
      cancelled = true;
      opened?.close();
    };
  }, [attempt, open]);

  useEffect(() => {
    if (replica === undefined) {
      return;
    }

    const active = replica;

    function refresh() {
      void active.reload().catch(() => {
        // the provider status surfaces storage failures
      });
    }

    function handleVisibility() {
      if (document.visibilityState === 'visible') {
        refresh();
      }
    }

    window.addEventListener('focus', refresh);
    window.addEventListener('pageshow', refresh);
    document.addEventListener('visibilitychange', handleVisibility);

    return () => {
      window.removeEventListener('focus', refresh);
      window.removeEventListener('pageshow', refresh);
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, [replica]);

  const retry = useCallback(() => {
    setStatus('loading');
    setError(undefined);
    setAttempt((current) => current + 1);
  }, []);

  const value = useMemo<ReplicaContextValue>(
    () => ({ status, replica, error, retry }),
    [status, replica, error, retry],
  );

  return <ReplicaContext.Provider value={value}>{children}</ReplicaContext.Provider>;
}
