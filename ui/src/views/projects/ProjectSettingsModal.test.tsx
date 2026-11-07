// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import type { Replica } from '@/utils/replica';
import { ProjectSettingsModal } from './ProjectSettingsModal';

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

function renderModal(enqueue: (operation: Operation) => Promise<void>, onClose = vi.fn()) {
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
      <ProjectSettingsModal project={projectSnapshot()} onClose={onClose} />
    </ReplicaContext.Provider>,
  );

  return { onClose };
}

function saveButton() {
  return screen.getByRole('button', { name: 'Save' }) as HTMLButtonElement;
}

afterEach(cleanup);

describe('ProjectSettingsModal', () => {
  it('starts with Save enabled for a valid project', () => {
    renderModal(vi.fn(async () => undefined));

    expect(saveButton().disabled).toBe(false);
  });

  it('disables Save when the name is empty', async () => {
    renderModal(vi.fn(async () => undefined));

    fireEvent.change(screen.getByLabelText(/^Name/), { target: { value: '   ' } });

    await waitFor(() => {
      expect(saveButton().disabled).toBe(true);
    });
  });

  it('re-enables Save when the name becomes valid again', async () => {
    renderModal(vi.fn(async () => undefined));

    fireEvent.change(screen.getByLabelText(/^Name/), { target: { value: '   ' } });
    await waitFor(() => {
      expect(saveButton().disabled).toBe(true);
    });

    fireEvent.change(screen.getByLabelText(/^Name/), { target: { value: 'Renamed' } });

    await waitFor(() => {
      expect(saveButton().disabled).toBe(false);
    });
  });

  it('disables Save while a save error is shown and clears the error on edit', async () => {
    const enqueue = vi.fn(async (_operation: Operation) => {
      throw new Error('offline');
    });
    renderModal(enqueue);

    fireEvent.change(screen.getByLabelText('Description'), { target: { value: 'Details' } });
    fireEvent.click(saveButton());

    expect(await screen.findByText('Could not save this project.')).toBeDefined();
    expect(saveButton().disabled).toBe(true);

    fireEvent.change(screen.getByLabelText('Description'), { target: { value: 'More details' } });

    await waitFor(() => {
      expect(screen.queryByText('Could not save this project.')).toBeNull();
    });
    await waitFor(() => {
      expect(saveButton().disabled).toBe(false);
    });
  });

  it('calls onClose when Cancel is clicked', () => {
    const { onClose } = renderModal(vi.fn(async () => undefined));

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
