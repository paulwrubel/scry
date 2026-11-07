import { describe, expect, it } from 'vitest';
import type { SyncSnapshot } from '@/utils/sync';
import { deriveSyncStatus, formatLastSyncedAt } from './useSyncStatus';

function snapshot(overrides: Partial<SyncSnapshot> = {}): SyncSnapshot {
  return {
    status: 'idle',
    error: null,
    rejected: [],
    lastSyncedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

const BASE = {
  isAuthenticated: true,
  isAuthLoading: false,
  pending: 0,
  syncingVisible: false,
  persisted: true,
};

describe('deriveSyncStatus', () => {
  it('prioritises checking and sign-in', () => {
    expect(deriveSyncStatus({ ...BASE, snapshot: snapshot(), isAuthLoading: true }).status).toBe(
      'checking',
    );
    expect(deriveSyncStatus({ ...BASE, snapshot: snapshot(), isAuthenticated: false }).status).toBe(
      'unauthenticated',
    );
  });

  it('shows syncing only once the debounce flag is set', () => {
    expect(
      deriveSyncStatus({ ...BASE, snapshot: snapshot({ status: 'syncing' }), syncingVisible: true })
        .status,
    ).toBe('syncing');
    expect(deriveSyncStatus({ ...BASE, snapshot: snapshot({ status: 'syncing' }) }).status).toBe(
      'synced',
    );
  });

  it('describes sync errors', () => {
    const error = Object.assign(new Error('down'), { kind: 'network' as const });
    const view = deriveSyncStatus({ ...BASE, snapshot: snapshot({ status: 'error', error }) });
    expect(view.status).toBe('error');
    expect(view.detail).toContain('Cannot reach');
  });

  it('treats an unauthorized error as signed out', () => {
    const error = Object.assign(new Error('401'), { kind: 'unauthorized' as const });
    expect(
      deriveSyncStatus({ ...BASE, snapshot: snapshot({ status: 'error', error }) }).status,
    ).toBe('unauthenticated');
  });

  it('reports pending work and rejected operations', () => {
    expect(deriveSyncStatus({ ...BASE, snapshot: snapshot(), pending: 2 }).status).toBe('pending');
    const rejected = deriveSyncStatus({
      ...BASE,
      snapshot: snapshot({ rejected: [{ operation_id: 'op', reason: 'not_found' }] }),
    });
    expect(rejected.status).toBe('error');
    expect(rejected.rejected).toBe(1);
    expect(rejected.detail).toContain('rejected');
  });

  it('reports the synced state and last sync time', () => {
    const view = deriveSyncStatus({ ...BASE, snapshot: snapshot() });
    expect(view.status).toBe('synced');
    expect(view.lastSyncedLabel).not.toBe('Never');
  });
});

describe('formatLastSyncedAt', () => {
  it('handles null, invalid, and valid values', () => {
    expect(formatLastSyncedAt(null)).toBe('Never');
    expect(formatLastSyncedAt('not a date')).toBe('Never');
    expect(formatLastSyncedAt('2026-01-01T00:00:00Z')).not.toBe('Never');
  });
});
