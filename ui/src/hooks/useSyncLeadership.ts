import { useCallback, useEffect, useRef, useState } from 'react';

/** Exclusive Web Lock name held by the one tab allowed to sync. */
export const SYNC_LOCK_NAME = 'scry-sync-leader';
/** BroadcastChannel the leader uses to tell followers that IndexedDB changed. */
export const SYNC_CHANNEL_NAME = 'scry-sync';

/**
 * Messages carry no state, only a hint about what changed. Followers re-read
 * the shared IndexedDB Replica rather than trusting data from another tab.
 */
export type SyncChannelMessage = { type: 'replica-updated' } | { type: 'outbox-changed' };

export type UseSyncLeadershipOptions = {
  onLeader: () => void;
  onMessage: (message: SyncChannelMessage) => void;
  /** Omit to use navigator.locks; pass null to simulate an unsupported browser. */
  lockManager?: LockManager | null;
  /** Omit to create a BroadcastChannel; pass null to simulate unsupported. */
  channel?: BroadcastChannel | null;
};

export type SyncLeadership = {
  isLeader: boolean;
  notify: (message: SyncChannelMessage) => void;
};

/**
 * Elect one sync leader per origin and exchange change notifications between
 * tabs.
 *
 * The lock callback holds its promise open for the tab's lifetime, so the
 * browser keeps the lock until this tab unmounts or closes. A second tab waits
 * in the lock queue and becomes leader when the first releases it.
 */
export function useSyncLeadership(options: UseSyncLeadershipOptions): SyncLeadership {
  const { onLeader, onMessage, lockManager, channel } = options;

  // Callers pass inline callbacks. Store the latest in refs so changing a
  // callback does not tear down the lock request or re-subscribe the channel.
  const onLeaderRef = useRef(onLeader);
  const onMessageRef = useRef(onMessage);
  useEffect(() => {
    onLeaderRef.current = onLeader;
  }, [onLeader]);
  useEffect(() => {
    onMessageRef.current = onMessage;
  }, [onMessage]);

  // Created in an effect so StrictMode's setup/cleanup cycle leaves a working
  // channel instead of reusing the one the previous cleanup closed.
  const [hasChannel] = useState(() => {
    if (channel !== undefined) {
      return channel !== null;
    }
    return typeof BroadcastChannel !== 'undefined';
  });
  const channelRef = useRef<BroadcastChannel | null>(null);

  // Without both APIs every tab leads. That is less efficient, but the server
  // dedupes operation ids and replay is idempotent, so it remains correct.
  const [isLeader, setIsLeader] = useState(() => {
    const locks = lockManager !== undefined ? lockManager : navigator.locks;
    return !locks || !hasChannel;
  });

  const notify = useCallback((message: SyncChannelMessage) => {
    channelRef.current?.postMessage(message);
  }, []);

  useEffect(() => {
    if (!hasChannel) {
      return;
    }

    const active = channel !== undefined ? channel : new BroadcastChannel(SYNC_CHANNEL_NAME);
    if (active === null) {
      return;
    }
    channelRef.current = active;

    function handleMessage(event: MessageEvent) {
      const data = event.data as unknown;
      // Another tab could post anything; ignore anything outside our protocol.
      if (isSyncChannelMessage(data)) {
        onMessageRef.current(data);
      }
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
    const locks = lockManager !== undefined ? lockManager : navigator.locks;

    // No coordination is available. Elect this tab immediately; the previous
    // state initialization already marked it leader.
    if (!locks || !hasChannel) {
      onLeaderRef.current();
      return;
    }

    let cancelled = false;
    let release: (() => void) | null = null;

    void locks
      .request(SYNC_LOCK_NAME, { mode: 'exclusive' }, async () => {
        // The effect was cleaned up while this request waited in the queue.
        if (cancelled) {
          return;
        }
        setIsLeader(true);
        onLeaderRef.current();

        // Hold the lock until unmount. Resolving this promise releases it and
        // lets the next queued tab run its callback.
        await new Promise<void>((resolve) => {
          release = resolve;
        });
        if (!cancelled) {
          setIsLeader(false);
        }
      })
      // A failed lock request should not disable sync; fall back to leading.
      .catch(() => {
        if (cancelled) {
          return;
        }
        setIsLeader(true);
        onLeaderRef.current();
      });

    return () => {
      cancelled = true;
      release?.();
    };
  }, [hasChannel, lockManager]);

  return { isLeader, notify };
}

function isSyncChannelMessage(value: unknown): value is SyncChannelMessage {
  if (typeof value !== 'object' || value === null || !('type' in value)) {
    return false;
  }
  const type = (value as { type: unknown }).type;
  return type === 'replica-updated' || type === 'outbox-changed';
}
