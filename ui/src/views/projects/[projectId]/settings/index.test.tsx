// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { ActiveProjectContext } from '@/providers/ActiveProject/context';
import { InstallPromptContext } from '@/providers/InstallPrompt/context';
import type { InstallPromptContextType } from '@/providers/InstallPrompt/context';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import { SyncContext } from '@/providers/Sync/context';
import type { SyncContextValue } from '@/providers/Sync/context';
import type { Replica } from '@/utils/replica';
import type { SyncClient, SyncSnapshot } from '@/utils/sync';
import { ProjectSettingsPage } from './index';

const CREATED_AT = '2026-01-01T00:00:00Z';

function projectSnapshot(id: string, name: string): ProjectSnapshot {
  return {
    id,
    name,
    description: null,
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
    sections: [],
    tasks: [],
    comments: [],
    automations: [],
  };
}

function LocationProbe() {
  const location = useLocation();
  return <span data-testid="path">{location.pathname}</span>;
}

function renderPage(
  projects: ProjectSnapshot[],
  activeProjectId: string | null,
  projectId: string,
  enqueue: (operation: Operation) => Promise<void>,
) {
  const snapshot = {
    confirmed: { projects },
    visible: { projects },
    outbox: [],
    needsBootstrap: false,
    persisted: true,
    lastSeenSeq: 0,
  };

  const replicaContext: ReplicaContextValue = {
    status: 'ready',
    replica: {
      getSnapshot: () => snapshot,
      subscribe: () => () => undefined,
      enqueue,
    } as unknown as Replica,
    error: undefined,
    retry: () => undefined,
  };

  const syncSnapshot: SyncSnapshot = {
    status: 'idle',
    error: null,
    rejected: [],
    lastSyncedAt: null,
  };
  const client: SyncClient = {
    getSnapshot: () => syncSnapshot,
    subscribe: () => () => undefined,
    sync: async () => undefined,
    clearRejectedOperation: () => undefined,
    clearAllRejectedOperations: () => undefined,
  };
  const syncContext: SyncContextValue = {
    client,
    syncNow: async () => undefined,
    clearRejectedOperation: () => undefined,
    clearAllRejectedOperations: () => undefined,
  };

  const installPromptContext: InstallPromptContextType = {
    canInstall: false,
    isInstalled: false,
    isIos: false,
    isMobile: false,
    promptInstall: async () => undefined,
  };

  const setActiveProjectId = vi.fn();
  const clearActiveProject = vi.fn();

  render(
    <ReplicaContext.Provider value={replicaContext}>
      <SyncContext.Provider value={syncContext}>
        <InstallPromptContext.Provider value={installPromptContext}>
          <ActiveProjectContext.Provider
            value={{ activeProjectId, setActiveProjectId, clearActiveProject }}
          >
            <MemoryRouter initialEntries={[`/projects/${projectId}/settings`]}>
              <LocationProbe />
              <Routes>
                <Route path="/projects/:projectId/settings" element={<ProjectSettingsPage />} />
              </Routes>
            </MemoryRouter>
          </ActiveProjectContext.Provider>
        </InstallPromptContext.Provider>
      </SyncContext.Provider>
    </ReplicaContext.Provider>,
  );

  return { setActiveProjectId, clearActiveProject };
}

afterEach(cleanup);

describe('ProjectSettingsPage', () => {
  it('renders the settings fields and delete action for an existing project', () => {
    renderPage(
      [projectSnapshot('project-1', 'Alpha')],
      'project-1',
      'project-1',
      vi.fn(async () => undefined),
    );

    expect(screen.getByLabelText(/^Name/)).toBeDefined();
    expect(screen.getByLabelText('Description')).toBeDefined();
    expect(screen.getByRole('button', { name: 'Delete this project' })).toBeDefined();
  });

  it('deletes the project and falls back to the next one', async () => {
    const enqueue = vi.fn(async () => undefined);
    const { setActiveProjectId, clearActiveProject } = renderPage(
      [projectSnapshot('project-1', 'Alpha'), projectSnapshot('project-2', 'Beta')],
      'project-1',
      'project-1',
      enqueue,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Delete this project' }));
    fireEvent.click(screen.getByRole('button', { name: 'Delete project' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({ kind: 'delete_project', id: 'project-1' }),
      );
    });
    await waitFor(() => {
      expect(setActiveProjectId).toHaveBeenCalledWith('project-2');
    });
    expect(clearActiveProject).not.toHaveBeenCalled();
    await waitFor(() => {
      expect(screen.getByTestId('path').textContent).toBe('/projects/project-2/tasks');
    });
  });
});
