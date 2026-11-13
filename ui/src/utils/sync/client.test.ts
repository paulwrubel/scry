import 'fake-indexeddb/auto';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ClientState } from '@/contract/op-replay';
import type { Change } from '@/generated/Change';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { openReplica } from '@/utils/replica';
import type { Replica } from '@/utils/replica';
import { createSyncClient } from './client';
import { SYNC_STATE_KEY } from './persistence';

const PROJECT_ID = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const TASK_ID = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const BASE_URL = 'http://test/api/v1';

let name = '';
let replica: Replica | null = null;

class MemoryStorage implements Storage {
  private readonly data = new Map<string, string>();

  get length(): number {
    return this.data.size;
  }

  clear(): void {
    this.data.clear();
  }

  getItem(key: string): string | null {
    return this.data.get(key) ?? null;
  }

  key(index: number): string | null {
    return [...this.data.keys()][index] ?? null;
  }

  removeItem(key: string): void {
    this.data.delete(key);
  }

  setItem(key: string, value: string): void {
    this.data.set(key, value);
  }
}

beforeEach(() => {
  name = `scry-sync-test-${crypto.randomUUID()}`;
  vi.stubGlobal('localStorage', new MemoryStorage());
});

afterEach(async () => {
  replica?.close();
  replica = null;
  await deleteDatabase(name);
  vi.unstubAllGlobals();
});

describe('sync client', () => {
  it('bootstraps from a snapshot', async () => {
    const testReplica = await openTestReplica();
    const project = projectSnapshot();
    const requests: string[] = [];
    const fetcher = createFetcher(
      {
        snapshot: [jsonResponse({ projects: [project], last_seen_seq: 5 })],
        sync: [jsonResponse({ acks: [], changes: [], next_seq: 5 })],
      },
      requests,
    );
    const client = createSyncClient({ replica: testReplica, fetcher, baseUrl: BASE_URL });

    await client.sync();

    expect(testReplica.confirmed.projects).toHaveLength(1);
    expect(testReplica.confirmed.projects[0].id).toBe(PROJECT_ID);
    expect(testReplica.lastSeenSeq).toBe(5);
    expect(testReplica.needsBootstrap).toBe(false);
    expect(client.getSnapshot().status).toBe('idle');
    expect(client.getSnapshot().lastSyncedAt).toEqual(expect.any(String));
    expect(requests[0]).toBe(`${BASE_URL}/snapshot`);
    expect(requests[1]).toBe(`${BASE_URL}/sync`);
  });

  it('applies pulled changes and advances the cursor', async () => {
    const testReplica = await openTestReplica();
    await testReplica.bootstrap({ projects: [projectSnapshot()] }, 0);

    const pulled = task();
    const change: Change = {
      seq: 1,
      entity_type: 'task',
      entity_id: pulled.id,
      kind: 'create',
      operation_id: 'op-pull',
      client_timestamp: null,
      field_values: { ...pulled },
      created_at: '2026-01-01T00:00:02Z',
    };

    const fetcher = createFetcher({
      sync: [jsonResponse({ acks: [], changes: [change], next_seq: 3 })],
    });
    const client = createSyncClient({ replica: testReplica, fetcher, baseUrl: BASE_URL });

    await client.sync();

    expect(findTask(testReplica.confirmed, TASK_ID)).toEqual(pulled);
    expect(findTask(testReplica.visible, TASK_ID)).toEqual(pulled);
    expect(testReplica.lastSeenSeq).toBe(3);
    expect(client.getSnapshot().status).toBe('idle');
  });

  it('acknowledges pushed operations', async () => {
    const testReplica = await openTestReplica();
    await testReplica.bootstrap({ projects: [] }, 0);

    const operation = createProjectOperation('op-create', 'project-create');
    await testReplica.enqueue(operation);

    const fetcher = createFetcher({
      sync: [
        jsonResponse({
          acks: [{ operation_id: 'op-create', status: 'applied' }],
          changes: [],
          next_seq: 0,
        }),
      ],
    });
    const client = createSyncClient({ replica: testReplica, fetcher, baseUrl: BASE_URL });

    await client.sync();

    expect(testReplica.outbox).toEqual([]);
    expect(client.getSnapshot().status).toBe('idle');
  });

  it('removes only acknowledged operations', async () => {
    const testReplica = await openTestReplica();
    await testReplica.bootstrap({ projects: [] }, 0);

    await testReplica.enqueue(createProjectOperation('op-ack', 'project-ack'));
    await testReplica.enqueue(createProjectOperation('op-pending', 'project-pending'));

    const fetcher = createFetcher({
      sync: [
        jsonResponse({
          acks: [{ operation_id: 'op-ack', status: 'applied' }],
          changes: [],
          next_seq: 0,
        }),
      ],
    });
    const client = createSyncClient({ replica: testReplica, fetcher, baseUrl: BASE_URL });

    await client.sync();

    expect(testReplica.outbox.map((operation) => operation.operation_id)).toEqual(['op-pending']);
  });

  it('surfaces rejected operations', async () => {
    const testReplica = await openTestReplica();
    await testReplica.bootstrap({ projects: [] }, 0);

    await testReplica.enqueue(createProjectOperation('op-rejected', 'project-rejected'));

    const fetcher = createFetcher({
      sync: [
        jsonResponse({
          acks: [{ operation_id: 'op-rejected', status: 'rejected', reason: 'not_found' }],
          changes: [],
          next_seq: 0,
        }),
      ],
    });
    const client = createSyncClient({ replica: testReplica, fetcher, baseUrl: BASE_URL });

    await client.sync();

    const snapshot = client.getSnapshot();
    expect(testReplica.outbox).toEqual([]);
    expect(snapshot.status).toBe('idle');
    expect(snapshot.rejected).toEqual([{ operation_id: 'op-rejected', reason: 'not_found' }]);
  });

  it('401 preserves the outbox', async () => {
    const testReplica = await openTestReplica();
    await testReplica.bootstrap({ projects: [] }, 0);

    await testReplica.enqueue(createProjectOperation('op-unauthorized', 'project-unauthorized'));

    const fetcher = createFetcher({
      sync: [new Response('unauthorized', { status: 401 })],
    });
    const client = createSyncClient({ replica: testReplica, fetcher, baseUrl: BASE_URL });

    await expect(client.sync()).rejects.toMatchObject({ name: 'SyncError', kind: 'unauthorized' });

    expect(client.getSnapshot().error?.kind).toBe('unauthorized');
    expect(testReplica.outbox.map((operation) => operation.operation_id)).toEqual([
      'op-unauthorized',
    ]);
  });

  it('request-level unsupported version preserves the outbox', async () => {
    const testReplica = await openTestReplica();
    await testReplica.bootstrap({ projects: [] }, 0);

    await testReplica.enqueue(createProjectOperation('op-unsupported', 'project-unsupported'));

    const fetcher = createFetcher({
      sync: [new Response('unsupported operation version', { status: 400 })],
    });
    const client = createSyncClient({ replica: testReplica, fetcher, baseUrl: BASE_URL });

    await expect(client.sync()).rejects.toMatchObject({
      name: 'SyncError',
      kind: 'unsupported_version',
    });

    expect(client.getSnapshot().error?.kind).toBe('unsupported_version');
    expect(testReplica.outbox.map((operation) => operation.operation_id)).toEqual([
      'op-unsupported',
    ]);
  });

  it('per-operation unsupported version preserves that operation', async () => {
    const testReplica = await openTestReplica();
    await testReplica.bootstrap({ projects: [] }, 0);

    await testReplica.enqueue(createProjectOperation('op-applied', 'project-applied'));
    await testReplica.enqueue(createProjectOperation('op-unsupported', 'project-unsupported'));

    const fetcher = createFetcher({
      sync: [
        jsonResponse({
          acks: [
            { operation_id: 'op-applied', status: 'applied' },
            { operation_id: 'op-unsupported', status: 'rejected', reason: 'unsupported_version' },
          ],
          changes: [],
          next_seq: 0,
        }),
      ],
    });
    const client = createSyncClient({ replica: testReplica, fetcher, baseUrl: BASE_URL });

    await expect(client.sync()).rejects.toMatchObject({
      name: 'SyncError',
      kind: 'unsupported_version',
    });

    expect(testReplica.outbox.map((operation) => operation.operation_id)).toEqual([
      'op-unsupported',
    ]);
    expect(client.getSnapshot().rejected).toEqual([
      { operation_id: 'op-unsupported', reason: 'unsupported_version' },
    ]);
    expect(client.getSnapshot().status).toBe('error');
  });

  it('network failure preserves the outbox', async () => {
    const testReplica = await openTestReplica();
    await testReplica.bootstrap({ projects: [] }, 0);

    await testReplica.enqueue(createProjectOperation('op-network', 'project-network'));

    const fetcher = (async () => {
      throw new TypeError('network down');
    }) as unknown as typeof fetch;
    const client = createSyncClient({ replica: testReplica, fetcher, baseUrl: BASE_URL });

    await expect(client.sync()).rejects.toMatchObject({
      name: 'SyncError',
      kind: 'network',
    });

    expect(client.getSnapshot().error?.kind).toBe('network');
    expect(testReplica.outbox.map((operation) => operation.operation_id)).toEqual(['op-network']);
  });

  it('keeps the last successful sync time when a later sync fails', async () => {
    const testReplica = await openTestReplica();
    await testReplica.bootstrap({ projects: [] }, 0);

    const fetcher = createFetcher({
      sync: [
        jsonResponse({ acks: [], changes: [], next_seq: 0 }),
        new Response('unauthorized', { status: 401 }),
      ],
    });
    const client = createSyncClient({ replica: testReplica, fetcher, baseUrl: BASE_URL });

    await client.sync();
    const lastSyncedAt = client.getSnapshot().lastSyncedAt;
    expect(lastSyncedAt).toEqual(expect.any(String));

    await expect(client.sync()).rejects.toMatchObject({ kind: 'unauthorized' });
    expect(client.getSnapshot().lastSyncedAt).toBe(lastSyncedAt);
  });

  it('persists rejections until dismissed', async () => {
    const testReplica = await openTestReplica();
    await testReplica.bootstrap({ projects: [] }, 0);

    await testReplica.enqueue(createProjectOperation('op-rejected', 'project-rejected'));

    const fetcher = createFetcher({
      sync: [
        jsonResponse({
          acks: [{ operation_id: 'op-rejected', status: 'rejected', reason: 'conflict' }],
          changes: [],
          next_seq: 0,
        }),
        jsonResponse({ acks: [], changes: [], next_seq: 0 }),
      ],
    });
    const client = createSyncClient({ replica: testReplica, fetcher, baseUrl: BASE_URL });

    await client.sync();
    expect(client.getSnapshot().rejected).toEqual([
      { operation_id: 'op-rejected', reason: 'conflict' },
    ]);

    // A clean sync must not erase the record.
    await client.sync();
    expect(client.getSnapshot().rejected).toEqual([
      { operation_id: 'op-rejected', reason: 'conflict' },
    ]);

    const reloaded = createSyncClient({
      replica: testReplica,
      fetcher: createFetcher({}),
      baseUrl: BASE_URL,
    });
    expect(reloaded.getSnapshot().rejected).toEqual([
      { operation_id: 'op-rejected', reason: 'conflict' },
    ]);
    expect(reloaded.getSnapshot().lastSyncedAt).toEqual(expect.any(String));

    reloaded.clearRejectedOperation('op-rejected');
    expect(reloaded.getSnapshot().rejected).toEqual([]);

    const afterDismiss = createSyncClient({
      replica: testReplica,
      fetcher: createFetcher({}),
      baseUrl: BASE_URL,
    });
    expect(afterDismiss.getSnapshot().rejected).toEqual([]);
  });

  it('falls back when the persisted state is malformed', async () => {
    localStorage.setItem(SYNC_STATE_KEY, '{not json');
    const testReplica = await openTestReplica();
    const client = createSyncClient({
      replica: testReplica,
      fetcher: createFetcher({}),
      baseUrl: BASE_URL,
    });

    expect(client.getSnapshot().rejected).toEqual([]);
    expect(client.getSnapshot().lastSyncedAt).toBeNull();
  });

  it('getSnapshot is stable and notifies', async () => {
    const testReplica = await openTestReplica();
    await testReplica.bootstrap({ projects: [] }, 0);

    const fetcher = createFetcher({
      sync: [jsonResponse({ acks: [], changes: [], next_seq: 0 })],
    });
    const client = createSyncClient({ replica: testReplica, fetcher, baseUrl: BASE_URL });

    const first = client.getSnapshot();
    expect(client.getSnapshot()).toBe(first);

    const listener = vi.fn();
    client.subscribe(listener);

    await client.sync();

    expect(listener).toHaveBeenCalled();
    const second = client.getSnapshot();
    expect(second).not.toBe(first);
    expect(second.status).toBe('idle');
  });
});

type QueuedResponses = {
  snapshot?: Response[];
  sync?: Response[];
};

function createFetcher(queued: QueuedResponses, requests: string[] = []): typeof fetch {
  const mock = vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    requests.push(url);
    if (url.endsWith('/snapshot')) {
      const response = queued.snapshot?.shift();
      if (response === undefined) {
        throw new Error('unexpected snapshot request');
      }
      return response;
    }
    if (url.endsWith('/sync')) {
      const response = queued.sync?.shift();
      if (response === undefined) {
        throw new Error('unexpected sync request');
      }
      return response;
    }
    throw new Error(`unexpected request: ${url}`);
  });

  return mock as unknown as typeof fetch;
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

async function openTestReplica(): Promise<Replica> {
  replica = await openReplica({ databaseName: name });
  return replica;
}

function createProjectOperation(operationId: string, id: string): Operation {
  return {
    operation_id: operationId,
    operation_version: 1,
    kind: 'create_project',
    id,
    name: 'Project',
    description: null,
    created_at: '2026-01-01T00:00:00Z',
  };
}

function projectSnapshot(overrides: Partial<ProjectSnapshot> = {}): ProjectSnapshot {
  return {
    id: PROJECT_ID,
    name: 'Project',
    description: null,
    created_at: '2026-01-01T00:00:00Z',
    sections: [],
    tasks: [],
    notes: [],
    automations: [],
    ...overrides,
  };
}

function task(overrides: Partial<Task> = {}): Task {
  return {
    id: TASK_ID,
    project_id: PROJECT_ID,
    title: 'Task',
    description: null,
    priority: 'medium',
    section_id: null,
    position: 'a0',
    state: 'open',
    tags: [],
    created_at: '2026-01-01T00:00:01Z',
    external_identity: null,
    external_metadata: null,
    ...overrides,
  };
}

function findTask(state: ClientState, id: string): Task | undefined {
  for (const project of state.projects) {
    const found = project.tasks.find((candidate) => candidate.id === id);
    if (found !== undefined) {
      return found;
    }
  }
  return undefined;
}

function deleteDatabase(databaseName: string): Promise<void> {
  return new Promise((resolve) => {
    const request = indexedDB.deleteDatabase(databaseName);
    request.onsuccess = () => resolve();
    request.onerror = () => resolve();
    request.onblocked = () => resolve();
  });
}
