// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { useState } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useCloseRequest } from './useCloseRequest';

class FakeCloseWatcher extends EventTarget {
  static instances: FakeCloseWatcher[] = [];

  destroyed = false;

  constructor() {
    super();
    FakeCloseWatcher.instances.push(this);
  }

  requestClose() {
    this.close();
  }

  close() {
    this.dispatchEvent(new Event('close'));
  }

  destroy() {
    this.destroyed = true;
    FakeCloseWatcher.instances = FakeCloseWatcher.instances.filter((instance) => instance !== this);
  }
}

function latestWatcher(): FakeCloseWatcher {
  const [watcher] = FakeCloseWatcher.instances;
  if (watcher === undefined) {
    throw new Error('expected a CloseWatcher to be created');
  }
  return watcher;
}

export type HarnessProps = {
  onClose: () => void;
};

export function Harness(props: HarnessProps) {
  const { onClose } = props;

  const [open, setOpen] = useState(false);
  useCloseRequest(open, () => {
    setOpen(false);
    onClose();
  });

  return (
    <div>
      <button type="button" onClick={() => setOpen(true)}>
        Open
      </button>
      <button type="button" onClick={() => setOpen(false)}>
        Close
      </button>
      {open ? <p>Overlay</p> : null}
    </div>
  );
}

beforeEach(() => {
  FakeCloseWatcher.instances = [];
  Object.defineProperty(globalThis, 'CloseWatcher', {
    configurable: true,
    writable: true,
    value: FakeCloseWatcher,
  });
});

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(globalThis, 'CloseWatcher');
});

describe('useCloseRequest', () => {
  it('creates a watcher only while open', () => {
    render(<Harness onClose={() => undefined} />);

    expect(FakeCloseWatcher.instances).toHaveLength(0);

    fireEvent.click(screen.getByRole('button', { name: 'Open' }));

    expect(FakeCloseWatcher.instances).toHaveLength(1);
  });

  it('closes the overlay when the watcher reports a close request', () => {
    const onClose = vi.fn();
    render(<Harness onClose={onClose} />);

    fireEvent.click(screen.getByRole('button', { name: 'Open' }));
    const watcher = latestWatcher();

    act(() => {
      watcher.close();
    });

    expect(screen.queryByText('Overlay')).toBeNull();
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('destroys the watcher when closed through the UI', () => {
    render(<Harness onClose={() => undefined} />);

    fireEvent.click(screen.getByRole('button', { name: 'Open' }));
    const watcher = latestWatcher();

    fireEvent.click(screen.getByRole('button', { name: 'Close' }));

    expect(watcher.destroyed).toBe(true);
  });

  it('destroys the watcher on unmount', () => {
    const { unmount } = render(<Harness onClose={() => undefined} />);

    fireEvent.click(screen.getByRole('button', { name: 'Open' }));
    const watcher = latestWatcher();

    unmount();

    expect(watcher.destroyed).toBe(true);
  });

  it('still opens when the platform API is unavailable', () => {
    Reflect.deleteProperty(globalThis, 'CloseWatcher');
    render(<Harness onClose={() => undefined} />);

    fireEvent.click(screen.getByRole('button', { name: 'Open' }));

    expect(screen.getByText('Overlay')).toBeDefined();
    expect(FakeCloseWatcher.instances).toHaveLength(0);
  });
});
