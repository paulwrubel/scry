import { useCallback, useEffect, useRef, useState } from 'react';
import { isSyncError } from '@/utils/sync';
import type { SyncClient } from '@/utils/sync';

export type SyncTiming = {
  editDebounceMs: number;
  pollIntervalMs: number;
  retryBaseMs: number;
  retryMaxMs: number;
};

export const SYNC_TIMING_DEFAULTS: SyncTiming = {
  editDebounceMs: 500,
  pollIntervalMs: 15_000,
  retryBaseMs: 1_000,
  retryMaxMs: 30_000,
};

export type UseSyncTriggersOptions = {
  client: SyncClient;
  outboxLength: number;
  timing?: Partial<SyncTiming>;
};

// Startup sync is application initialization, not per-mount state. Tracking
// clients keeps StrictMode remounts and provider remounts from syncing twice.
const startedClients = new WeakSet<SyncClient>();

export function useSyncTriggers(options: UseSyncTriggersOptions): () => Promise<void> {
  const { client, outboxLength, timing } = options;
  const editDebounceMs = timing?.editDebounceMs ?? SYNC_TIMING_DEFAULTS.editDebounceMs;
  const pollIntervalMs = timing?.pollIntervalMs ?? SYNC_TIMING_DEFAULTS.pollIntervalMs;
  const retryBaseMs = timing?.retryBaseMs ?? SYNC_TIMING_DEFAULTS.retryBaseMs;
  const retryMaxMs = timing?.retryMaxMs ?? SYNC_TIMING_DEFAULTS.retryMaxMs;

  const retryTimer = useRef<number | null>(null);
  const retryDelay = useRef(retryBaseMs);
  const runRef = useRef<() => Promise<void>>(async () => {
    // replaced on every render
  });
  const [visible, setVisible] = useState(() => document.visibilityState === 'visible');

  const clearRetry = useCallback(() => {
    if (retryTimer.current !== null) {
      window.clearTimeout(retryTimer.current);
      retryTimer.current = null;
    }
  }, []);

  const scheduleRetry = useCallback(() => {
    if (retryTimer.current !== null || !navigator.onLine) {
      return;
    }
    const delay = retryDelay.current;
    retryDelay.current = Math.min(delay * 2, retryMaxMs);
    retryTimer.current = window.setTimeout(() => {
      retryTimer.current = null;
      void runRef.current();
    }, delay);
  }, [retryMaxMs]);

  const runSync = useCallback(async () => {
    if (!navigator.onLine) {
      return;
    }
    try {
      await client.sync();
      retryDelay.current = retryBaseMs;
      clearRetry();
    } catch (error) {
      if (
        isSyncError(error) &&
        (error.kind === 'unauthorized' || error.kind === 'unsupported_version')
      ) {
        return;
      }
      scheduleRetry();
    }
  }, [client, clearRetry, retryBaseMs, scheduleRetry]);

  useEffect(() => {
    runRef.current = runSync;
  }, [runSync]);

  const syncNow = useCallback(async () => {
    clearRetry();
    retryDelay.current = retryBaseMs;
    try {
      await client.sync();
    } catch {
      // the sync client exposes the failure in its snapshot
    }
  }, [client, clearRetry, retryBaseMs]);

  // startup: once per client, not once per component mount
  useEffect(() => {
    if (startedClients.has(client)) {
      return;
    }
    startedClients.add(client);
    void runSync();
  }, [client, runSync]);

  // edit debounce: only when the outbox grows
  const previousOutboxLength = useRef(outboxLength);
  useEffect(() => {
    const previous = previousOutboxLength.current;
    previousOutboxLength.current = outboxLength;
    if (outboxLength <= previous) {
      return;
    }
    const timer = window.setTimeout(() => {
      void runSync();
    }, editDebounceMs);
    return () => window.clearTimeout(timer);
  }, [editDebounceMs, outboxLength, runSync]);

  // visibility: sync immediately when the tab becomes visible
  useEffect(() => {
    function handleVisibilityChange() {
      const nextVisible = document.visibilityState === 'visible';
      setVisible(nextVisible);
      if (nextVisible) {
        void runSync();
      }
    }
    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => document.removeEventListener('visibilitychange', handleVisibilityChange);
  }, [runSync]);

  // visible polling; hidden tabs do not poll
  useEffect(() => {
    if (!visible) {
      return;
    }
    const timer = window.setInterval(() => {
      void runSync();
    }, pollIntervalMs);
    return () => window.clearInterval(timer);
  }, [pollIntervalMs, runSync, visible]);

  // online/offline
  useEffect(() => {
    function handleOnline() {
      retryDelay.current = retryBaseMs;
      clearRetry();
      void runSync();
    }
    function handleOffline() {
      clearRetry();
    }
    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);
    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, [clearRetry, retryBaseMs, runSync]);

  // cleanup any pending retry on unmount
  useEffect(() => () => clearRetry(), [clearRetry]);

  return syncNow;
}
