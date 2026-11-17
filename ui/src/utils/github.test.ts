import { DateTime } from 'luxon';
import { describe, expect, it } from 'vitest';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { ExternalMetadata } from '@/generated/ExternalMetadata';
import type { Task } from '@/generated/Task';
import { githubSyncStale, lastSyncedAt } from './github';

const NOW = DateTime.fromISO('2026-03-01T12:00:00Z', { setZone: true });

const LINKED: ExternalIdentity = {
  provider: 'github',
  owner: 'acme',
  repo: 'widgets',
  number: 123,
};

function metadata(syncedAt: string | null): ExternalMetadata {
  return {
    provider: 'github',
    title: null,
    description: null,
    kind: 'issue',
    state: 'open',
    reason: null,
    draft: null,
    updated_at: null,
    synced_at: syncedAt,
    error: null,
  };
}

function task(overrides: Partial<Task> = {}): Task {
  return {
    id: 'task-1',
    project_id: 'project-1',
    title: 'Task',
    description: null,
    priority: 'normal',
    section_id: null,
    position: 'a0',
    state: 'open',
    closed_reason: null,
    closed_at: null,
    tags: [],
    due: null,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    external_identity: LINKED,
    external_metadata: metadata('2026-03-01T11:00:00Z'),
    ...overrides,
  };
}

describe('lastSyncedAt', () => {
  it('returns the metadata sync time', () => {
    expect(lastSyncedAt(task())).toBe('2026-03-01T11:00:00Z');
  });

  it('returns null when never synced or unlinked', () => {
    expect(lastSyncedAt(task({ external_metadata: metadata(null) }))).toBeNull();
    expect(lastSyncedAt(task({ external_identity: null, external_metadata: null }))).toBeNull();
  });
});

describe('githubSyncStale', () => {
  it('ignores unlinked tasks', () => {
    expect(githubSyncStale(task({ external_identity: null, external_metadata: null }), NOW)).toBe(
      false,
    );
  });

  it('treats a missing sync time as stale', () => {
    expect(githubSyncStale(task({ external_metadata: metadata(null) }), NOW)).toBe(true);
  });

  it('is false within the 24 hour window', () => {
    const syncedAt = NOW.minus({ hours: 23 }).toISO() ?? '';

    expect(githubSyncStale(task({ external_metadata: metadata(syncedAt) }), NOW)).toBe(false);
  });

  it('is true once the window is exceeded', () => {
    const syncedAt = NOW.minus({ hours: 25 }).toISO() ?? '';

    expect(githubSyncStale(task({ external_metadata: metadata(syncedAt) }), NOW)).toBe(true);
  });

  it('treats an unparseable timestamp as stale', () => {
    expect(githubSyncStale(task({ external_metadata: metadata('not-a-date') }), NOW)).toBe(true);
  });
});
