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
  enabled?: boolean;
};

// Startup sync is application initialization, not per-mount state. Tracking
// clients keeps StrictMode remounts and provider remounts from syncing twice.
const startedClients = new WeakSet<SyncClient>();

export function useSyncTriggers(options: UseSyncTriggersOptions): () => Promise<void> {
  const {
    client,
    outboxLength,
    enabled = true,
    timing: {
      editDebounceMs = SYNC_TIMING_DEFAULTS.editDebounceMs,
      pollIntervalMs = SYNC_TIMING_DEFAULTS.pollIntervalMs,
      retryBaseMs = SYNC_TIMING_DEFAULTS.retryBaseMs,
      retryMaxMs = SYNC_TIMING_DEFAULTS.retryMaxMs,
    } = {},
  } = options;

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
    if (!enabled || !navigator.onLine) {
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
  }, [client, clearRetry, enabled, retryBaseMs, scheduleRetry]);

  useEffect(() => {
    runRef.current = runSync;
  }, [runSync]);

  const syncNow = useCallback(async () => {
    if (!enabled) {
      return;
    }
    clearRetry();
    retryDelay.current = retryBaseMs;
    try {
      await client.sync();
    } catch {
      // the sync client exposes the failure in its snapshot
    }
  }, [client, clearRetry, enabled, retryBaseMs]);

  // startup: once per client, not once per component mount
  useEffect(() => {
    if (!enabled || startedClients.has(client)) {
      return;
    }
    startedClients.add(client);
    void runSync();
  }, [client, enabled, runSync]);

  // edit debounce: only when the outbox grows
  const previousOutboxLength = useRef(outboxLength);
  useEffect(() => {
    const previous = previousOutboxLength.current;
    previousOutboxLength.current = outboxLength;
    if (!enabled || outboxLength <= previous) {
      return;
    }
    const timer = window.setTimeout(() => {
      void runSync();
    }, editDebounceMs);
    return () => window.clearTimeout(timer);
  }, [editDebounceMs, enabled, outboxLength, runSync]);

  // visibility: sync immediately when the tab becomes visible
  useEffect(() => {
    if (!enabled) {
      return;
    }
    function handleVisibilityChange() {
      const nextVisible = document.visibilityState === 'visible';
      setVisible(nextVisible);
      if (nextVisible) {
        void runSync();
      }
    }
    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => document.removeEventListener('visibilitychange', handleVisibilityChange);
  }, [enabled, runSync]);

  // visible polling; hidden tabs do not poll
  useEffect(() => {
    if (!visible || !enabled) {
      return;
    }
    const timer = window.setInterval(() => {
      void runSync();
    }, pollIntervalMs);
    return () => window.clearInterval(timer);
  }, [enabled, pollIntervalMs, runSync, visible]);

  // online/offline
  useEffect(() => {
    if (!enabled) {
      return;
    }
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
  }, [clearRetry, enabled, retryBaseMs, runSync]);

  // a tab that loses leadership must not keep a retry queued
  useEffect(() => {
    if (!enabled) {
      clearRetry();
    }
  }, [clearRetry, enabled]);

  // cleanup any pending retry on unmount
  useEffect(() => () => clearRetry(), [clearRetry]);

  return syncNow;
}
