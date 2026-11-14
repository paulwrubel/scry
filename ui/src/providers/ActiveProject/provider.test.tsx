// @vitest-environment jsdom
import 'fake-indexeddb/auto';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import type { ClientState } from '@/contract/replay';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { ReplicaProvider, useReplicaStatus } from '@/providers/Replica';
import { openReplica } from '@/utils/replica';
import { useActiveProject } from './hook';
import { ActiveProjectProvider } from './provider';
import {
  ACTIVE_PROJECT_STORAGE_KEY,
  readActiveProjectId,
  resolveActiveProjectId,
  writeActiveProjectId,
} from '@/utils/projects/activeProject';

const PROJECT_A_ID = '11111111-1111-4111-8111-111111111111';
const PROJECT_B_ID = '22222222-2222-4222-8222-222222222222';

let name = '';

beforeEach(() => {
  name = `scry-active-project-test-${crypto.randomUUID()}`;
  localStorage.clear();
});

afterEach(async () => {
  cleanup();
  localStorage.clear();
  await deleteDatabase(name);
});

describe('resolveActiveProjectId', () => {
  it('keeps a preferred project that exists', () => {
    expect(resolveActiveProjectId(['a', 'b'], 'b')).toBe('b');
  });

  it('falls back to the first project when the preferred one is stale', () => {
    expect(resolveActiveProjectId(['a', 'b'], 'missing')).toBe('a');
  });

  it('returns null when there are no projects', () => {
    expect(resolveActiveProjectId([], 'missing')).toBeNull();
  });
});

describe('active project storage', () => {
  it('round-trips and removes a stored project id', () => {
    writeActiveProjectId('project-1');
    expect(readActiveProjectId()).toBe('project-1');
    expect(localStorage.getItem(ACTIVE_PROJECT_STORAGE_KEY)).toBe('project-1');

    writeActiveProjectId(null);
    expect(readActiveProjectId()).toBeNull();
    expect(localStorage.getItem(ACTIVE_PROJECT_STORAGE_KEY)).toBeNull();
  });
});

describe('ActiveProjectProvider', () => {
  it('falls back to the first project and persists a selection across mounts', async () => {
    const open = async () => {
      const replica = await openReplica({ databaseName: name });
      await replica.bootstrap(twoProjectState());
      return replica;
    };

    const first = render(
      <MemoryRouter>
        <ReplicaProvider open={open}>
          <ReadyGate>
            <ActiveProjectProvider>
              <Probe />
            </ActiveProjectProvider>
          </ReadyGate>
        </ReplicaProvider>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('active').textContent).toBe(PROJECT_A_ID);
    });

    fireEvent.click(screen.getByRole('button', { name: 'select second' }));

    expect(localStorage.getItem(ACTIVE_PROJECT_STORAGE_KEY)).toBe(PROJECT_B_ID);

    first.unmount();

    render(
      <MemoryRouter>
        <ReplicaProvider open={open}>
          <ReadyGate>
            <ActiveProjectProvider>
              <Probe />
            </ActiveProjectProvider>
          </ReadyGate>
        </ReplicaProvider>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('active').textContent).toBe(PROJECT_B_ID);
    });
  });
});

type ReadyGateProps = {
  children: React.ReactNode;
};

function ReadyGate(props: ReadyGateProps) {
  const { children } = props;
  const { status } = useReplicaStatus();
  return status === 'ready' ? children : null;
}

function Probe() {
  const { activeProjectId, setActiveProjectId, clearActiveProject } = useActiveProject();

  return (
    <div>
      <span data-testid="active">{activeProjectId ?? 'none'}</span>
      <button type="button" onClick={() => setActiveProjectId(PROJECT_B_ID)}>
        select second
      </button>
      <button type="button" onClick={clearActiveProject}>
        clear
      </button>
    </div>
  );
}

function projectSnapshot(id: string, projectName: string): ProjectSnapshot {
  return {
    id,
    name: projectName,
    description: null,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    sections: [],
    tasks: [],
    comments: [],
    automations: [],
  };
}

function twoProjectState(): ClientState {
  return {
    projects: [projectSnapshot(PROJECT_A_ID, 'Alpha'), projectSnapshot(PROJECT_B_ID, 'Beta')],
  };
}

function deleteDatabase(databaseName: string): Promise<void> {
  return new Promise((resolve) => {
    const request = indexedDB.deleteDatabase(databaseName);
    request.onsuccess = () => resolve();
    request.onerror = () => resolve();
    request.onblocked = () => resolve();
  });
}
