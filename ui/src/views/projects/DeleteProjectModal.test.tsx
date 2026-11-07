// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import type { Replica } from '@/utils/replica';
import { DeleteProjectModal } from './DeleteProjectModal';

const PROJECT_ID = 'project-1';
const CREATED_AT = '2026-01-01T00:00:00Z';

function projectSnapshot(): ProjectSnapshot {
  return {
    id: PROJECT_ID,
    name: 'Project',
    description: null,
    task_sorting_mode: 'manual',
    show_priority: false,
    created_at: CREATED_AT,
    sections: [],
    tasks: [],
    notes: [],
  };
}

function renderModal(
  enqueue: (operation: Operation) => Promise<void>,
  onDeleted = vi.fn(),
  onClose = vi.fn(),
) {
  const snapshot = {
    confirmed: { projects: [projectSnapshot()] },
    visible: { projects: [projectSnapshot()] },
    outbox: [],
    needsBootstrap: false,
    persisted: true,
    lastSeenSeq: 0,
  };

  const context: ReplicaContextValue = {
    status: 'ready',
    replica: {
      getSnapshot: () => snapshot,
      subscribe: () => () => undefined,
      enqueue,
    } as unknown as Replica,
    error: undefined,
    retry: () => undefined,
  };

  render(
    <ReplicaContext.Provider value={context}>
      <DeleteProjectModal project={projectSnapshot()} onClose={onClose} onDeleted={onDeleted} />
    </ReplicaContext.Provider>,
  );

  return { onDeleted, onClose };
}

afterEach(cleanup);

describe('DeleteProjectModal', () => {
  it('renders a confirmation that names the project', () => {
    renderModal(vi.fn(async () => undefined));

    expect(screen.getByText(/permanently deletes the project/)).toBeDefined();
    expect(screen.getByText(/Project/)).toBeDefined();
  });

  it('enqueues a delete_project operation and reports the deleted project', async () => {
    const enqueue = vi.fn(async () => undefined);
    const { onDeleted } = renderModal(enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Delete project' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({ kind: 'delete_project', id: PROJECT_ID }),
      );
    });
    await waitFor(() => {
      expect(onDeleted).toHaveBeenCalledWith(PROJECT_ID);
    });
  });

  it('closes without enqueueing when cancelled', () => {
    const enqueue = vi.fn(async () => undefined);
    const { onClose, onDeleted } = renderModal(enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(enqueue).not.toHaveBeenCalled();
    expect(onDeleted).not.toHaveBeenCalled();
  });

  it('shows an error and stays open when the enqueue fails', async () => {
    const enqueue = vi.fn(async () => {
      throw new Error('offline');
    });
    const { onDeleted, onClose } = renderModal(enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Delete project' }));

    expect(await screen.findByText('Could not delete this project.')).toBeDefined();
    expect(onDeleted).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });
});
