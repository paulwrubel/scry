// @vitest-environment jsdom
import { act, cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DeleteSectionModal } from './DeleteSectionModal';

class FakeCloseWatcher extends EventTarget {
  static instances: FakeCloseWatcher[] = [];

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
    FakeCloseWatcher.instances = FakeCloseWatcher.instances.filter((instance) => instance !== this);
  }
}

function renderModal(blockingAutomations: readonly string[], show = true) {
  const onClose = vi.fn();

  render(
    <DeleteSectionModal
      sectionName="Todo"
      blockingAutomations={blockingAutomations}
      show={show}
      onClose={onClose}
      onDelete={vi.fn()}
    />,
  );

  return { onClose };
}

function deleteButton() {
  return screen.getByRole('button', { name: 'Delete section' }) as HTMLButtonElement;
}

afterEach(cleanup);

describe('DeleteSectionModal', () => {
  it('enables delete when no automation blocks the section', () => {
    renderModal([]);

    expect(deleteButton().disabled).toBe(false);
  });

  it('disables delete and lists the blocking rules', () => {
    renderModal(['When a task enters Todo → move to Doing']);

    expect(deleteButton().disabled).toBe(true);
    expect(screen.getByText('This section is used by automations.')).toBeDefined();
    expect(screen.getByText('When a task enters Todo → move to Doing')).toBeDefined();
  });

  describe('platform close requests', () => {
    beforeEach(() => {
      FakeCloseWatcher.instances = [];
      Object.defineProperty(globalThis, 'CloseWatcher', {
        configurable: true,
        writable: true,
        value: FakeCloseWatcher,
      });
    });

    afterEach(() => {
      Reflect.deleteProperty(globalThis, 'CloseWatcher');
    });

    it('closes on a platform close request while open', () => {
      const { onClose } = renderModal([]);
      const [watcher] = FakeCloseWatcher.instances;
      if (watcher === undefined) {
        throw new Error('expected a CloseWatcher to be created');
      }

      act(() => {
        watcher.close();
      });

      expect(onClose).toHaveBeenCalledTimes(1);
    });

    it('creates no watcher while closed', () => {
      renderModal([], false);

      expect(FakeCloseWatcher.instances).toHaveLength(0);
    });
  });
});
