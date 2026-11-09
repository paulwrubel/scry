// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { InstallPromptContext } from '@/providers/InstallPrompt/context';
import type { InstallPromptContextType } from '@/providers/InstallPrompt/context';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import { SyncContext } from '@/providers/Sync/context';
import type { SyncContextValue } from '@/providers/Sync/context';
import type { Replica } from '@/utils/replica';
import type { RejectedOperation, SyncClient, SyncSnapshot } from '@/utils/sync';
import { AppSection } from './AppSection';

afterEach(cleanup);

function renderSection(
  options: {
    persisted?: boolean;
    rejected?: readonly RejectedOperation[];
    install?: Partial<InstallPromptContextType>;
  } = {},
) {
  const { persisted = true, rejected = [], install = {} } = options;

  const snapshot = {
    confirmed: { projects: [] },
    visible: { projects: [] },
    outbox: [],
    needsBootstrap: false,
    persisted,
    lastSeenSeq: 0,
  };

  const replicaContext: ReplicaContextValue = {
    status: 'ready',
    replica: {
      getSnapshot: () => snapshot,
      subscribe: () => () => undefined,
      enqueue: async () => undefined,
    } as unknown as Replica,
    error: undefined,
    retry: () => undefined,
  };

  const syncSnapshot: SyncSnapshot = {
    status: 'idle',
    error: null,
    rejected,
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
    promptInstall: vi.fn(async () => undefined),
    ...install,
  };

  render(
    <ReplicaContext.Provider value={replicaContext}>
      <SyncContext.Provider value={syncContext}>
        <InstallPromptContext.Provider value={installPromptContext}>
          <AppSection />
        </InstallPromptContext.Provider>
      </SyncContext.Provider>
    </ReplicaContext.Provider>,
  );
}

describe('AppSection', () => {
  it('renders nothing when there are no diagnostics or install options', () => {
    renderSection();

    expect(screen.queryByRole('heading', { name: 'App' })).toBeNull();
  });

  it('renders the storage warning when local data is not persistent', () => {
    renderSection({ persisted: false });

    expect(screen.getByRole('heading', { name: 'App' })).toBeDefined();
    expect(screen.getByText('Local storage is not persistent')).toBeDefined();
  });

  it('renders the install card when the browser can install the app', () => {
    renderSection({ install: { canInstall: true } });

    expect(screen.getByRole('heading', { name: 'App' })).toBeDefined();
    expect(screen.getByRole('button', { name: 'Install Scry' })).toBeDefined();
  });
});
