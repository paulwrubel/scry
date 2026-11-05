// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, renderHook, screen } from '@testing-library/react';
import { StrictMode } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useSyncLeadership } from './useSyncLeadership';

class FakeLockManager {
  private held = false;
  private queue: Array<() => void> = [];

  request(
    _name: string,
    _options: { mode: 'exclusive' },
    callback: (lock: Lock | null) => Promise<void>,
  ): Promise<void> {
    const run = () => {
      this.held = true;
      void callback(null).finally(() => {
        this.held = false;
        this.queue.shift()?.();
      });
    };
    if (this.held) {
      this.queue.push(run);
    } else {
      run();
    }
    return Promise.resolve();
  }
}

class FakeChannel {
  private listeners = new Set<(event: MessageEvent) => void>();
  peers: FakeChannel[] = [];

  addEventListener(_type: 'message', listener: (event: MessageEvent) => void) {
    this.listeners.add(listener);
  }
  removeEventListener(_type: 'message', listener: (event: MessageEvent) => void) {
    this.listeners.delete(listener);
  }
  postMessage(data: unknown) {
    for (const peer of this.peers) {
      for (const listener of peer.listeners) {
        listener({ data } as MessageEvent);
      }
    }
  }
  close() {
    // no transport to close in the fake
  }
}

function createLocks(): LockManager {
  return new FakeLockManager() as unknown as LockManager;
}

function createChannel(): FakeChannel {
  return new FakeChannel();
}

function link(first: FakeChannel, second: FakeChannel) {
  first.peers.push(second);
  second.peers.push(first);
}

async function flush() {
  await act(() => Promise.resolve());
}

class ClosableChannel {
  readonly messages: unknown[] = [];
  closed = false;
  private listeners = new Set<(event: MessageEvent) => void>();

  addEventListener(_type: 'message', listener: (event: MessageEvent) => void) {
    this.listeners.add(listener);
  }

  removeEventListener(_type: 'message', listener: (event: MessageEvent) => void) {
    this.listeners.delete(listener);
  }

  postMessage(data: unknown) {
    if (this.closed) {
      throw new DOMException('the channel is closed', 'InvalidStateError');
    }
    this.messages.push(data);
  }

  close() {
    this.closed = true;
  }
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe('useSyncLeadership', () => {
  it('elects one leader and transfers on cleanup', async () => {
    const locks = createLocks();
    const channelA = createChannel();
    const channelB = createChannel();
    link(channelA, channelB);
    const onLeaderA = vi.fn();
    const onLeaderB = vi.fn();

    const first = renderHook(() =>
      useSyncLeadership({
        onLeader: onLeaderA,
        onMessage: vi.fn(),
        lockManager: locks,
        channel: channelA as unknown as BroadcastChannel,
      }),
    );
    const second = renderHook(() =>
      useSyncLeadership({
        onLeader: onLeaderB,
        onMessage: vi.fn(),
        lockManager: locks,
        channel: channelB as unknown as BroadcastChannel,
      }),
    );
    await flush();

    expect(first.result.current.isLeader).toBe(true);
    expect(second.result.current.isLeader).toBe(false);
    expect(onLeaderA).toHaveBeenCalledTimes(1);

    first.unmount();
    await flush();
    await flush();

    expect(second.result.current.isLeader).toBe(true);
    expect(onLeaderB).toHaveBeenCalledTimes(1);
  });

  it('routes channel messages', async () => {
    const locks = createLocks();
    const channelA = createChannel();
    const channelB = createChannel();
    link(channelA, channelB);
    const onMessageA = vi.fn();
    const onMessageB = vi.fn();

    const first = renderHook(() =>
      useSyncLeadership({
        onLeader: vi.fn(),
        onMessage: onMessageA,
        lockManager: locks,
        channel: channelA as unknown as BroadcastChannel,
      }),
    );
    renderHook(() =>
      useSyncLeadership({
        onLeader: vi.fn(),
        onMessage: onMessageB,
        lockManager: locks,
        channel: channelB as unknown as BroadcastChannel,
      }),
    );
    await flush();

    act(() => {
      first.result.current.notify({ type: 'outbox-changed' });
    });
    await flush();

    expect(onMessageB).toHaveBeenCalledWith({ type: 'outbox-changed' });
    expect(onMessageA).not.toHaveBeenCalled();
  });

  it('falls back to leader without locks', async () => {
    const onLeader = vi.fn();

    const { result } = renderHook(() =>
      useSyncLeadership({ onLeader, onMessage: vi.fn(), lockManager: null, channel: null }),
    );
    await flush();

    expect(result.current.isLeader).toBe(true);
    expect(onLeader).toHaveBeenCalledTimes(1);
  });

  it('calls onLeader when leadership is acquired', async () => {
    const onLeader = vi.fn();
    const locks = createLocks();
    const channel = createChannel();

    renderHook(() =>
      useSyncLeadership({
        onLeader,
        onMessage: vi.fn(),
        lockManager: locks,
        channel: channel as unknown as BroadcastChannel,
      }),
    );
    await flush();

    expect(onLeader).toHaveBeenCalledTimes(1);
  });

  it('keeps a usable channel after the StrictMode cleanup', () => {
    const channels: ClosableChannel[] = [];
    vi.stubGlobal(
      'BroadcastChannel',
      class {
        constructor() {
          const channel = new ClosableChannel();
          channels.push(channel);
          return channel;
        }
      },
    );

    function StrictProbe() {
      const { notify } = useSyncLeadership({
        onLeader: () => undefined,
        onMessage: () => undefined,
      });
      return <button onClick={() => notify({ type: 'replica-updated' })}>notify</button>;
    }

    render(
      <StrictMode>
        <StrictProbe />
      </StrictMode>,
    );

    expect(channels.length).toBeGreaterThan(1);
    expect(channels[0].closed).toBe(true);

    fireEvent.click(screen.getByRole('button', { name: 'notify' }));

    expect(channels[channels.length - 1].messages).toEqual([{ type: 'replica-updated' }]);
  });

  it('ignores unknown channel payloads', async () => {
    const sender = createChannel();
    const channel = createChannel();
    sender.peers.push(channel);
    const onMessage = vi.fn();

    renderHook(() =>
      useSyncLeadership({
        onLeader: vi.fn(),
        onMessage,
        lockManager: createLocks(),
        channel: channel as unknown as BroadcastChannel,
      }),
    );
    await flush();

    act(() => {
      sender.postMessage({ type: 'not-a-message' });
      sender.postMessage('garbage');
    });
    await flush();

    expect(onMessage).not.toHaveBeenCalled();
  });
});
