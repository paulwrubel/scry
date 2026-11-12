import 'fake-indexeddb/auto';
import { openDB } from 'idb';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ClientState } from '@/contract/op-replay';
import type { Automation } from '@/generated/Automation';
import type { Change } from '@/generated/Change';
import type { Note } from '@/generated/Note';
import type { Operation } from '@/generated/Operation';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import { openReplica } from './replica';
import type { ReplicaDB } from './storage';
import {
  CONFIRMED_SCHEMA_VERSION,
  REPLICA_DATABASE_VERSION,
  isReplicaError,
  replicaError,
} from './types';

const PROJECT_ID = 'project-1';
const SECTION_A_ID = 'section-a';
const SECTION_B_ID = 'section-b';
const TASK_A_ID = 'task-a';
const TASK_B_ID = 'task-b';
const NOTE_ID = 'note-a';
const AUTOMATION_ID = 'automation-a';

let name = '';

beforeEach(() => {
  name = `scry-replica-test-${crypto.randomUUID()}`;
});

afterEach(async () => {
  await deleteDatabase(name);
});

describe('replica confirmed state', () => {
  it('stores entities keyed by uuid and hydrates them', async () => {
    const state = sampleState();
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(state);

    expect(replica.confirmed).toEqual(state);
    replica.close();

    const db = await openDB<ReplicaDB>(name, REPLICA_DATABASE_VERSION);
    const project = state.projects[0];
    const { sections, tasks, notes, automations, ...projectRecord } = project;

    expect(await db.get('projects', project.id)).toEqual(projectRecord);
    expect(await db.get('sections', SECTION_A_ID)).toEqual(sections[0]);
    expect(await db.get('sections', SECTION_B_ID)).toEqual(sections[1]);
    expect(await db.get('tasks', TASK_A_ID)).toEqual(tasks[0]);
    expect(await db.get('tasks', TASK_B_ID)).toEqual(tasks[1]);
    expect(await db.get('notes', NOTE_ID)).toEqual(notes[0]);
    expect(automations).toEqual([]);
    expect(await db.get('meta', 'project_order')).toEqual([PROJECT_ID]);
    expect(await db.get('meta', 'section_order')).toEqual([SECTION_A_ID, SECTION_B_ID]);
    expect(await db.get('meta', 'task_order')).toEqual([TASK_A_ID, TASK_B_ID]);
    expect(await db.get('meta', 'note_order')).toEqual([NOTE_ID]);
    expect(await db.get('meta', 'schema_version')).toBe(CONFIRMED_SCHEMA_VERSION);
    db.close();

    const reopened = await openReplica({ databaseName: name });
    expect(reopened.confirmed).toEqual(state);
    reopened.close();
  });

  it('reload preserves confirmed state', async () => {
    const state = sampleState();
    const first = await openReplica({ databaseName: name });
    await first.bootstrap(state);
    first.close();

    const second = await openReplica({ databaseName: name });
    expect(second.confirmed).toEqual(state);
    second.close();
  });

  it('persists an automation and attaches it to its project', async () => {
    const automation: Automation = {
      id: AUTOMATION_ID,
      project_id: PROJECT_ID,
      trigger: { kind: 'task_created' },
      actions: [{ kind: 'close_task' }],
      created_at: '2026-01-01T00:00:05Z',
    };
    const state = sampleState();
    state.projects[0].automations = [automation];

    const first = await openReplica({ databaseName: name });
    await first.bootstrap(state);
    first.close();

    const db = await openDB<ReplicaDB>(name, REPLICA_DATABASE_VERSION);
    expect(await db.get('automations', AUTOMATION_ID)).toEqual(automation);
    db.close();

    const second = await openReplica({ databaseName: name });
    expect(second.confirmed.projects[0].automations).toEqual([automation]);
    second.close();
  });

  it('bootstrapping replaces older confirmed state', async () => {
    const firstState = sampleState();
    const secondState: ClientState = {
      projects: [
        {
          id: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
          name: 'Second project',
          description: null,
          task_sorting_mode: 'manual',
          show_priority: false,
          created_at: '2026-02-01T00:00:00Z',
          sections: [],
          tasks: [],
          notes: [],
          automations: [],
        },
      ],
    };

    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(firstState);
    await replica.bootstrap(secondState);
    replica.close();

    const reopened = await openReplica({ databaseName: name });
    expect(reopened.confirmed).toEqual(secondState);
    reopened.close();
  });

  it('order is preserved even when uuid key order differs', async () => {
    const state = orderedState();
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(state);
    replica.close();

    const reopened = await openReplica({ databaseName: name });
    const projects = reopened.confirmed.projects;

    expect(projects.map((project) => project.id)).toEqual(['zzz-project', 'aaa-project']);
    expect(projects[0].sections.map((section) => section.id)).toEqual([
      'zzz-section',
      'aaa-section',
    ]);
    expect(projects[0].tasks.map((task) => task.id)).toEqual(['zzz-task', 'aaa-task']);
    reopened.close();
  });
});

describe('replica outbox and visible state', () => {
  it('outbox is fifo and survives reload', async () => {
    const state = sampleState();
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(state);

    const created = extraTask('task-c', 'Created task');
    await replica.enqueue(createTaskOperation('op-create', created));
    await replica.enqueue(updateTaskOperation('op-update', TASK_A_ID, 'Renamed task A'));
    await replica.enqueue(deleteTaskOperation('op-delete', TASK_A_ID));

    expect(replica.outbox.map((operation) => operation.operation_id)).toEqual([
      'op-create',
      'op-update',
      'op-delete',
    ]);
    expect(findTask(replica.visible, 'task-c')).toEqual(created);
    expect(findTask(replica.visible, TASK_A_ID)).toBeUndefined();
    replica.close();

    const reopened = await openReplica({ databaseName: name });
    expect(reopened.outbox.map((operation) => operation.operation_id)).toEqual([
      'op-create',
      'op-update',
      'op-delete',
    ]);
    expect(findTask(reopened.visible, TASK_A_ID)).toBeUndefined();
    reopened.close();
  });

  it('offline edits update visible state immediately', async () => {
    const state = sampleState();
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(state);

    const created = extraTask('task-c', 'Created task');
    await replica.enqueue(createTaskOperation('op-create', created));
    expect(findTask(replica.visible, 'task-c')).toEqual(created);

    await replica.enqueue(updateTaskOperation('op-update', 'task-c', 'Updated task'));
    expect(findTask(replica.visible, 'task-c')?.title).toBe('Updated task');

    await replica.enqueue(deleteTaskOperation('op-delete', 'task-c'));
    expect(findTask(replica.visible, 'task-c')).toBeUndefined();

    expect(replica.confirmed).toEqual(state);
    replica.close();
  });

  it('acknowledged operations leave the outbox', async () => {
    const state = sampleState();
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(state);

    const created = extraTask('task-c', 'Created task');
    await replica.enqueue(createTaskOperation('op-create', created));
    expect(findTask(replica.visible, 'task-c')).toEqual(created);

    await replica.acknowledge(['op-create']);
    expect(replica.outbox).toEqual([]);
    expect(replica.visible).toEqual(replica.confirmed);
    replica.close();
  });

  it('apply changes updates confirmed and visible', async () => {
    const state = sampleState();
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(state);

    const applied = extraTask('task-applied', 'Applied task');
    const change: Change = {
      seq: 1,
      entity_type: 'task',
      entity_id: applied.id,
      kind: 'create',
      operation_id: 'op-applied',
      client_timestamp: null,
      field_values: applied,
      created_at: '2026-01-01T00:00:06Z',
    };

    await replica.applySyncResult([change], 1);
    expect(findTask(replica.confirmed, 'task-applied')).toEqual(applied);
    expect(findTask(replica.visible, 'task-applied')).toEqual(applied);
    replica.close();

    const reopened = await openReplica({ databaseName: name });
    expect(findTask(reopened.confirmed, 'task-applied')).toEqual(applied);
    reopened.close();
  });

  it('snapshots are stable until mutation and notify subscribers', async () => {
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(sampleState());

    const first = replica.getSnapshot();
    expect(replica.getSnapshot()).toBe(first);

    const listener = vi.fn();
    const unsubscribe = replica.subscribe(listener);

    const created = extraTask('task-c', 'Created task');
    await replica.enqueue(createTaskOperation('op-create', created));

    expect(listener).toHaveBeenCalledTimes(1);
    const second = replica.getSnapshot();
    expect(second).not.toBe(first);
    expect(findTask(second.visible, 'task-c')).toEqual(created);

    unsubscribe();
    await replica.enqueue(updateTaskOperation('op-update', 'task-c', 'Updated task'));
    expect(listener).toHaveBeenCalledTimes(1);
    replica.close();
  });

  it('reload preserves pending outbox and visible state', async () => {
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(sampleState());
    await replica.enqueue(updateTaskOperation('op-update', TASK_A_ID, 'Persisted title'));
    replica.close();

    const reopened = await openReplica({ databaseName: name });
    expect(reopened.outbox.map((operation) => operation.operation_id)).toEqual(['op-update']);
    expect(findTask(reopened.visible, TASK_A_ID)?.title).toBe('Persisted title');
    reopened.close();
  });

  it("reload sees another replica's confirmed state and outbox", async () => {
    const first = await openReplica({ databaseName: name });
    const second = await openReplica({ databaseName: name });

    await first.bootstrap(sampleState(), 5);
    await first.enqueue(updateTaskOperation('op-update', TASK_A_ID, 'Another tab title'));
    await second.reload();

    expect(second.confirmed).toEqual(first.confirmed);
    expect(second.outbox).toEqual(first.outbox);
    expect(second.lastSeenSeq).toBe(first.lastSeenSeq);
    expect(findTask(second.visible, TASK_A_ID)?.title).toBe('Another tab title');
    first.close();
    second.close();
  });
});

describe('replica sync cursor', () => {
  it('cursor persists across reopen', async () => {
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(sampleState(), 12);
    replica.close();

    const reopened = await openReplica({ databaseName: name });
    expect(reopened.lastSeenSeq).toBe(12);
    expect(reopened.getSnapshot().lastSeenSeq).toBe(12);
    reopened.close();
  });

  it('bootstrap sets the cursor', async () => {
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(sampleState(), 7);

    expect(replica.lastSeenSeq).toBe(7);
    replica.close();
  });

  it('applySyncResult persists the cursor', async () => {
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(sampleState());
    await replica.applySyncResult([], 9);
    replica.close();

    const reopened = await openReplica({ databaseName: name });
    expect(reopened.lastSeenSeq).toBe(9);
    reopened.close();
  });

  it('reload updates the cursor', async () => {
    const first = await openReplica({ databaseName: name });
    const second = await openReplica({ databaseName: name });

    await first.bootstrap(sampleState(), 0);
    await first.applySyncResult([], 12);
    await second.reload();

    expect(second.lastSeenSeq).toBe(12);
    expect(second.getSnapshot().lastSeenSeq).toBe(12);
    first.close();
    second.close();
  });

  it('reload updates needsBootstrap', async () => {
    const state = sampleState();
    const first = await openReplica({ databaseName: name });
    const second = await openReplica({ databaseName: name });

    expect(second.confirmed).toEqual({ projects: [] });
    expect(second.needsBootstrap).toBe(true);

    await first.bootstrap(state);
    await second.reload();

    expect(second.confirmed).toEqual(state);
    expect(second.needsBootstrap).toBe(false);
    first.close();
    second.close();
  });

  it('a stale response does not regress the cursor or the state', async () => {
    const first = await openReplica({ databaseName: name });
    const second = await openReplica({ databaseName: name });

    await first.bootstrap(sampleState(), 0);
    await second.reload();

    await second.applySyncResult([taskTitleChange(10, 'newer')], 10);
    const result = await first.applySyncResult([taskTitleChange(5, 'older')], 5);

    expect(result.applied).toBe(false);
    expect(first.lastSeenSeq).toBe(10);
    expect(findTask(first.confirmed, TASK_A_ID)?.title).toBe('newer');
    first.close();
    second.close();
  });

  it('applies only changes after the stored cursor', async () => {
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(sampleState(), 0);

    await replica.applySyncResult(
      [taskTitleChange(5, 'five'), taskTitleChange(14, 'fourteen')],
      14,
    );

    expect(findTask(replica.confirmed, TASK_A_ID)?.title).toBe('fourteen');
    expect(replica.lastSeenSeq).toBe(14);
    replica.close();
  });

  it('schema bump resets the cursor', async () => {
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(sampleState(), 12);
    await replica.enqueue(updateTaskOperation('op-update', TASK_A_ID, 'Persisted title'));
    replica.close();

    const db = await openDB<ReplicaDB>(name, REPLICA_DATABASE_VERSION);
    await db.put('meta', 0, 'schema_version');
    db.close();

    const reopened = await openReplica({ databaseName: name });
    expect(reopened.confirmed).toEqual({ projects: [] });
    expect(reopened.needsBootstrap).toBe(true);
    expect(reopened.lastSeenSeq).toBe(0);
    expect(reopened.outbox.map((operation) => operation.operation_id)).toEqual(['op-update']);
    reopened.close();
  });
});

describe('replica schema migration', () => {
  it('schema bump clears confirmed state but preserves the outbox', async () => {
    const state = sampleState();
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(state);
    await replica.enqueue(updateTaskOperation('op-update', TASK_A_ID, 'Persisted title'));
    replica.close();

    const db = await openDB<ReplicaDB>(name, REPLICA_DATABASE_VERSION);
    await db.put('meta', 0, 'schema_version');
    db.close();

    const reopened = await openReplica({ databaseName: name });
    expect(reopened.confirmed).toEqual({ projects: [] });
    expect(reopened.needsBootstrap).toBe(true);
    expect(reopened.outbox.map((operation) => operation.operation_id)).toEqual(['op-update']);
    reopened.close();

    const check = await openDB<ReplicaDB>(name, REPLICA_DATABASE_VERSION);
    expect(await check.getAll('projects')).toEqual([]);
    expect(await check.getAll('tasks')).toEqual([]);
    expect(await check.getAll('outbox')).toHaveLength(1);
    check.close();

    const bootstrapped = await openReplica({ databaseName: name });
    await bootstrapped.bootstrap(state);
    expect(bootstrapped.needsBootstrap).toBe(false);
    expect(bootstrapped.outbox.map((operation) => operation.operation_id)).toEqual(['op-update']);
    bootstrapped.close();
  });

  it('old operation versions survive reopen', async () => {
    const replica = await openReplica({ databaseName: name });
    await replica.bootstrap(sampleState());
    await replica.enqueue({
      ...updateTaskOperation('op-old', TASK_A_ID, 'Old operation'),
      operation_version: 1,
    });
    replica.close();

    const reopened = await openReplica({ databaseName: name });
    expect(reopened.outbox[0].operation_version).toBe(1);
    reopened.close();
  });
});

describe('replica persistence', () => {
  it('persistence denial is non-fatal', async () => {
    const persist = vi.fn(async () => false);
    vi.stubGlobal('navigator', { storage: { persist } });

    const replica = await openReplica({ databaseName: name });
    expect(persist).toHaveBeenCalled();
    expect(replica.persisted).toBe(false);
    expect(replica.confirmed).toEqual({ projects: [] });
    replica.close();
    vi.unstubAllGlobals();
  });

  it('persistence grant is reported', async () => {
    const persist = vi.fn(async () => true);
    vi.stubGlobal('navigator', { storage: { persist } });

    const replica = await openReplica({ databaseName: name });
    expect(persist).toHaveBeenCalled();
    expect(replica.persisted).toBe(true);
    replica.close();
    vi.unstubAllGlobals();
  });
});

describe('replica errors', () => {
  it('replica errors are tagged', () => {
    const error = replicaError('quota', 'full');
    expect(isReplicaError(error)).toBe(true);
    expect(error.kind).toBe('quota');
    expect(error).toBeInstanceOf(Error);
  });
});

function createTaskOperation(operationId: string, task: Task): Operation {
  return {
    operation_id: operationId,
    operation_version: 1,
    kind: 'create_task',
    id: task.id,
    project_id: task.project_id,
    title: task.title,
    description: task.description,
    priority: task.priority,
    section_id: task.section_id,
    position: task.position,
    state: task.state,
    tags: task.tags,
    created_at: task.created_at,
  };
}

function updateTaskOperation(operationId: string, taskId: string, title: string): Operation {
  return {
    operation_id: operationId,
    operation_version: 1,
    kind: 'update_task',
    id: taskId,
    title,
    description: null,
    priority: null,
    tags: null,
    state: null,
  };
}

function deleteTaskOperation(operationId: string, taskId: string): Operation {
  return {
    operation_id: operationId,
    operation_version: 1,
    kind: 'delete_task',
    id: taskId,
  };
}

function taskTitleChange(seq: number, title: string): Change {
  return {
    seq,
    entity_type: 'task',
    entity_id: TASK_A_ID,
    kind: 'update',
    operation_id: `op-${seq}`,
    client_timestamp: null,
    field_values: { title },
    created_at: '2026-01-01T00:00:06Z',
  };
}

function extraTask(id: string, title: string): Task {
  return {
    id,
    project_id: PROJECT_ID,
    title,
    description: null,
    priority: 'medium',
    section_id: null,
    position: 'a2',
    state: 'open',
    tags: [],
    created_at: '2026-01-01T00:00:05Z',
    external_identity: null,
    external_metadata: null,
  };
}

function findTask(state: ClientState, id: string): Task | undefined {
  for (const project of state.projects) {
    const task = project.tasks.find((candidate) => candidate.id === id);
    if (task !== undefined) {
      return task;
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

function sampleState(): ClientState {
  const sectionA: Section = {
    id: SECTION_A_ID,
    project_id: PROJECT_ID,
    name: 'Section A',
    position: 'a0',
    color: 'blue',
    created_at: '2026-01-01T00:00:00Z',
  };
  const sectionB: Section = {
    id: SECTION_B_ID,
    project_id: PROJECT_ID,
    name: 'Section B',
    position: 'a1',
    color: null,
    created_at: '2026-01-01T00:00:01Z',
  };
  const taskA: Task = {
    id: TASK_A_ID,
    project_id: PROJECT_ID,
    title: 'Task A',
    description: 'The first task',
    priority: 'high',
    section_id: SECTION_A_ID,
    position: 'a0',
    state: 'open',
    tags: ['alpha', 'beta'],
    created_at: '2026-01-01T00:00:02Z',
    external_identity: null,
    external_metadata: null,
  };
  const taskB: Task = {
    id: TASK_B_ID,
    project_id: PROJECT_ID,
    title: 'Task B',
    description: null,
    priority: 'low',
    section_id: SECTION_B_ID,
    position: 'a1',
    state: 'closed',
    tags: [],
    created_at: '2026-01-01T00:00:03Z',
    external_identity: null,
    external_metadata: null,
  };
  const note: Note = {
    id: NOTE_ID,
    task_id: TASK_A_ID,
    contents: 'A note',
    created_at: '2026-01-01T00:00:04Z',
  };

  return {
    projects: [
      {
        id: PROJECT_ID,
        name: 'Project',
        description: 'The project',
        task_sorting_mode: 'manual',
        show_priority: true,
        created_at: '2026-01-01T00:00:00Z',
        sections: [sectionA, sectionB],
        tasks: [taskA, taskB],
        notes: [note],
        automations: [],
      },
    ],
  };
}

function orderedState(): ClientState {
  const sectionA: Section = {
    id: 'zzz-section',
    project_id: 'zzz-project',
    name: 'First section',
    position: 'a0',
    color: 'green',
    created_at: '2026-03-01T00:00:00Z',
  };
  const sectionB: Section = {
    id: 'aaa-section',
    project_id: 'zzz-project',
    name: 'Second section',
    position: 'a1',
    color: null,
    created_at: '2026-03-01T00:00:01Z',
  };
  const taskA: Task = {
    id: 'zzz-task',
    project_id: 'zzz-project',
    title: 'First task',
    description: null,
    priority: 'low',
    section_id: 'zzz-section',
    position: 'a0',
    state: 'open',
    tags: ['first'],
    created_at: '2026-03-01T00:00:02Z',
    external_identity: null,
    external_metadata: null,
  };
  const taskB: Task = {
    id: 'aaa-task',
    project_id: 'zzz-project',
    title: 'Second task',
    description: 'second',
    priority: 'critical',
    section_id: 'aaa-section',
    position: 'a1',
    state: 'closed',
    tags: [],
    created_at: '2026-03-01T00:00:03Z',
    external_identity: null,
    external_metadata: null,
  };
  const note: Note = {
    id: 'zzz-note',
    task_id: 'zzz-task',
    contents: 'Note text',
    created_at: '2026-03-01T00:00:04Z',
  };

  return {
    projects: [
      {
        id: 'zzz-project',
        name: 'First project',
        description: null,
        task_sorting_mode: 'manual',
        show_priority: true,
        created_at: '2026-03-01T00:00:00Z',
        sections: [sectionA, sectionB],
        tasks: [taskA, taskB],
        notes: [note],
        automations: [],
      },
      {
        id: 'aaa-project',
        name: 'Second project',
        description: 'empty',
        task_sorting_mode: 'alphabetical',
        show_priority: false,
        created_at: '2026-03-01T00:00:05Z',
        sections: [],
        tasks: [],
        notes: [],
        automations: [],
      },
    ],
  };
}
