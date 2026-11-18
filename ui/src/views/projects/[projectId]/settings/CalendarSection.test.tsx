// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { AuthContextType, LoginResult } from '@/providers/Auth/context';
import { AuthContext } from '@/providers/Auth/context';
import type { SyncContextValue } from '@/providers/Sync/context';
import { SyncContext } from '@/providers/Sync/context';
import { makeEvent, makeProject } from '@/test/fixtures';
import type { SyncClient, SyncSnapshot } from '@/utils/sync';
import { CalendarSection } from './CalendarSection';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

type RenderOptions = {
  fetch: typeof fetch;
};

function renderSection(options: RenderOptions) {
  const syncNow = vi.fn(async () => undefined);

  const authContext: AuthContextType = {
    accessToken: 'test-token',
    isAuthenticated: true,
    isAuthLoading: false,
    authenticatedFetch: options.fetch,
    login: async (): Promise<LoginResult> => ({ ok: true }),
    logout: async () => undefined,
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
    syncNow,
    clearRejectedOperation: () => undefined,
    clearAllRejectedOperations: () => undefined,
  };

  render(
    <AuthContext.Provider value={authContext}>
      <SyncContext.Provider value={syncContext}>
        <CalendarSection project={makeProject()} />
      </SyncContext.Provider>
    </AuthContext.Provider>,
  );

  return { syncNow };
}

function selectFile(text: string) {
  const input = screen.getByLabelText('Calendar file');
  const file = new File([text], 'calendar.ics', { type: 'text/calendar' });
  fireEvent.change(input, { target: { files: [file] } });
}

describe('CalendarSection', () => {
  it('imports a calendar and syncs the replica', async () => {
    const event = makeEvent();
    const authenticatedFetch = vi.fn(
      async () =>
        new Response(JSON.stringify([event]), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
    );
    const { syncNow } = renderSection({ fetch: authenticatedFetch });

    selectFile('BEGIN:VCALENDAR');

    expect(await screen.findByText('Imported 1 event.')).toBeDefined();
    expect(syncNow).toHaveBeenCalledTimes(1);
  });

  it('shows the server message when the import fails', async () => {
    const authenticatedFetch = vi.fn(
      async () =>
        new Response(JSON.stringify({ message: 'Bad calendar' }), {
          status: 400,
          headers: { 'Content-Type': 'application/json' },
        }),
    );
    renderSection({ fetch: authenticatedFetch });

    selectFile('not a calendar');

    expect(await screen.findByText('Bad calendar')).toBeDefined();
  });

  it('downloads the exported calendar', async () => {
    const createObjectURL = vi.fn(() => 'blob:calendar');
    const revokeObjectURL = vi.fn();
    vi.stubGlobal('URL', { createObjectURL, revokeObjectURL });
    const anchorClick = vi
      .spyOn(HTMLAnchorElement.prototype, 'click')
      .mockImplementation(() => undefined);

    const authenticatedFetch = vi.fn(async () => new Response('BEGIN:VCALENDAR', { status: 200 }));
    renderSection({ fetch: authenticatedFetch });

    fireEvent.click(screen.getByRole('button', { name: 'Export calendar' }));

    expect(await screen.findByText('Exported Project.ics.')).toBeDefined();
    expect(createObjectURL).toHaveBeenCalledTimes(1);
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:calendar');
    expect(anchorClick).toHaveBeenCalledTimes(1);
  });
});
