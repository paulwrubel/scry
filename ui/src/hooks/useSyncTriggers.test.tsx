// @vitest-environment jsdom
import { StrictMode } from 'react';
import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { SyncClient } from '@/utils/sync';
import { syncError } from '@/utils/sync';
import { useSyncTriggers } from './useSyncTriggers';

const TIMING = {
  editDebounceMs: 100,
  pollIntervalMs: 1000,
  retryBaseMs: 100,
  retryMaxMs: 400,
};

function createClient(sync: SyncClient['sync']): SyncClient {
  return {
    sync,
    getSnapshot: () => ({ status: 'idle', error: null, rejected: [], lastSyncedAt: null }),
    subscribe: () => () => undefined,
    clearRejectedOperation: () => undefined,
    clearAllRejectedOperations: () => undefined,
  };
}

function setVisibility(state: DocumentVisibilityState) {
  Object.defineProperty(document, 'visibilityState', {
    configurable: true,
    get: () => state,
  });
}

let online = true;

beforeEach(() => {
  vi.useFakeTimers();
  online = true;
  Object.defineProperty(navigator, 'onLine', {
    configurable: true,
    get: () => online,
  });
  setVisibility('visible');
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  Reflect.deleteProperty(navigator, 'onLine');
  Reflect.deleteProperty(document, 'visibilityState');
});

async function flush() {
  await act(() => Promise.resolve());
}

async function advance(ms: number) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
  await flush();
}

describe('useSyncTriggers', () => {
  it('syncs on startup', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    renderHook(() => useSyncTriggers({ client, outboxLength: 0, timing: TIMING }));
    await flush();

    expect(sync).toHaveBeenCalledTimes(1);
  });

  it('syncs once under StrictMode', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    renderHook(() => useSyncTriggers({ client, outboxLength: 0, timing: TIMING }), {
      wrapper: ({ children }) => <StrictMode>{children}</StrictMode>,
    });
    await flush();

    expect(sync).toHaveBeenCalledTimes(1);
  });

  it('debounces an edit until the outbox stops growing', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    const { rerender } = renderHook(
      ({ outboxLength }: { outboxLength: number }) =>
        useSyncTriggers({ client, outboxLength, timing: TIMING }),
      { initialProps: { outboxLength: 0 } },
    );
    await flush();
    expect(sync).toHaveBeenCalledTimes(1);

    rerender({ outboxLength: 1 });
    await advance(99);
    expect(sync).toHaveBeenCalledTimes(1);

    await advance(1);
    expect(sync).toHaveBeenCalledTimes(2);
  });

  it('polls while the tab is visible', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    renderHook(() => useSyncTriggers({ client, outboxLength: 0, timing: TIMING }));
    await flush();
    expect(sync).toHaveBeenCalledTimes(1);

    await advance(1000);
    expect(sync).toHaveBeenCalledTimes(2);
  });

  it('does not poll while the tab is hidden', async () => {
    setVisibility('hidden');
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    renderHook(() => useSyncTriggers({ client, outboxLength: 0, timing: TIMING }));
    await flush();
    expect(sync).toHaveBeenCalledTimes(1);

    await advance(2000);
    expect(sync).toHaveBeenCalledTimes(1);
  });

  it('syncs when the tab becomes visible', async () => {
    setVisibility('hidden');
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    renderHook(() => useSyncTriggers({ client, outboxLength: 0, timing: TIMING }));
    await flush();
    expect(sync).toHaveBeenCalledTimes(1);

    setVisibility('visible');
    await act(async () => {
      document.dispatchEvent(new Event('visibilitychange'));
    });
    await flush();

    expect(sync).toHaveBeenCalledTimes(2);
  });

  it('syncs when the browser comes online', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    renderHook(() => useSyncTriggers({ client, outboxLength: 0, timing: TIMING }));
    await flush();
    expect(sync).toHaveBeenCalledTimes(1);

    await act(async () => {
      window.dispatchEvent(new Event('online'));
    });
    await flush();

    expect(sync).toHaveBeenCalledTimes(2);
  });

  it('backs off exponentially and resets after a success', async () => {
    const sync = vi
      .fn()
      .mockRejectedValueOnce(syncError('network', 'down'))
      .mockRejectedValueOnce(syncError('network', 'down'))
      .mockResolvedValueOnce(undefined);
    const client = createClient(sync);

    renderHook(() => useSyncTriggers({ client, outboxLength: 0, timing: TIMING }));
    await flush();
    expect(sync).toHaveBeenCalledTimes(1);

    await advance(100);
    expect(sync).toHaveBeenCalledTimes(2);

    await advance(200);
    expect(sync).toHaveBeenCalledTimes(3);

    await advance(400);
    expect(sync).toHaveBeenCalledTimes(3);

    sync.mockRejectedValue(syncError('network', 'down'));
    await act(async () => {
      window.dispatchEvent(new Event('online'));
    });
    await flush();
    expect(sync).toHaveBeenCalledTimes(4);

    await advance(99);
    expect(sync).toHaveBeenCalledTimes(4);

    await advance(1);
    expect(sync).toHaveBeenCalledTimes(5);
  });

  it('does not retry an unauthorized failure', async () => {
    setVisibility('hidden');
    const sync = vi.fn().mockRejectedValue(syncError('unauthorized', 'no'));
    const client = createClient(sync);

    renderHook(() => useSyncTriggers({ client, outboxLength: 0, timing: TIMING }));
    await flush();
    expect(sync).toHaveBeenCalledTimes(1);

    await advance(2000);
    expect(sync).toHaveBeenCalledTimes(1);
  });

  it('does not sync while disabled', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    const client = createClient(sync);

    const { rerender } = renderHook(
      ({ enabled }: { enabled: boolean }) =>
        useSyncTriggers({ client, outboxLength: 0, timing: TIMING, enabled }),
      { initialProps: { enabled: false } },
    );
    await flush();
    expect(sync).not.toHaveBeenCalled();

    rerender({ enabled: true });
    await flush();
    expect(sync).toHaveBeenCalledTimes(1);
  });

  it('exposes a manual sync that resolves even when syncing fails', async () => {
    const sync = vi.fn().mockRejectedValue(new Error('boom'));
    const client = createClient(sync);

    const { result } = renderHook(() =>
      useSyncTriggers({ client, outboxLength: 0, timing: TIMING }),
    );
    await flush();
    expect(sync).toHaveBeenCalledTimes(1);

    await act(async () => {
      await expect(result.current()).resolves.toBeUndefined();
    });

    expect(sync).toHaveBeenCalledTimes(2);
  });
});
