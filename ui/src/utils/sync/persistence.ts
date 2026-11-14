import type { RejectedOperation } from './types';

export const SYNC_STATE_KEY = 'scry.sync.state';

export type PersistedSyncState = {
  lastSyncedAt: string | null;
  rejected: RejectedOperation[];
};

const EMPTY: PersistedSyncState = { lastSyncedAt: null, rejected: [] };

function storage(): Storage | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
}

export function loadSyncState(): PersistedSyncState {
  const store = storage();
  if (store === null) {
    return { ...EMPTY };
  }

  try {
    const raw = store.getItem(SYNC_STATE_KEY);
    if (raw === null) {
      return { ...EMPTY };
    }
    const parsed: unknown = JSON.parse(raw);
    return isPersistedSyncState(parsed) ? parsed : { ...EMPTY };
  } catch {
    return { ...EMPTY };
  }
}

export function saveSyncState(state: PersistedSyncState): void {
  const store = storage();
  if (store === null) {
    return;
  }

  try {
    store.setItem(SYNC_STATE_KEY, JSON.stringify(state));
  } catch {
    // persistence is best-effort; the in-memory snapshot stays authoritative
  }
}

function isPersistedSyncState(value: unknown): value is PersistedSyncState {
  if (typeof value !== 'object' || value === null) {
    return false;
  }

  // the guard above proved value is a non-null object, but TS cannot index unknown
  const record = value as Record<string, unknown>;
  if (record.lastSyncedAt !== null && typeof record.lastSyncedAt !== 'string') {
    return false;
  }
  if (!Array.isArray(record.rejected)) {
    return false;
  }

  return record.rejected.every((item) => {
    if (typeof item !== 'object' || item === null) {
      return false;
    }
    // same guard as above: item is a non-null object, cast only to index its fields
    const rejected = item as Record<string, unknown>;
    return typeof rejected.operation_id === 'string' && isRejectionReason(rejected.reason);
  });
}

function isRejectionReason(value: unknown): value is RejectedOperation['reason'] {
  return (
    value === 'tombstoned' ||
    value === 'not_found' ||
    value === 'conflict' ||
    value === 'unsupported_version'
  );
}
