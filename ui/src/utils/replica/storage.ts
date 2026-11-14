import type { DBSchema, IDBPDatabase, IDBPTransaction } from 'idb';
import { openDB } from 'idb';
import type { ClientState } from '@/contract/replay';
import { applyChanges as applyConfirmedChanges } from '@/contract/replay';
import type { Automation } from '@/generated/Automation';
import type { Change } from '@/generated/Change';
import type { Comment } from '@/generated/Comment';
import type { Operation } from '@/generated/Operation';
import type { Project } from '@/generated/Project';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import type { ReplicaError } from './types';
import {
  CONFIRMED_SCHEMA_VERSION,
  REPLICA_DATABASE_VERSION,
  isReplicaError,
  replicaError,
} from './types';

type StoredOutboxEntry = { seq?: number; operation: Operation };

export type ReplicaDB = {
  projects: { key: string; value: Project };
  sections: { key: string; value: Section };
  tasks: { key: string; value: Task };
  comments: { key: string; value: Comment };
  automations: { key: string; value: Automation };
  outbox: {
    key: number;
    value: StoredOutboxEntry;
    indexes: { operation_id: string };
  };
  meta: { key: string; value: unknown };
} & DBSchema;

export type LoadedConfirmed = {
  state: ClientState;
  needsBootstrap: boolean;
  lastSeenSeq: number;
};

export type AppliedSyncResult = {
  applied: boolean;
  state: ClientState;
  lastSeenSeq: number;
};

const CONFIRMED_STORES = [
  'projects',
  'sections',
  'tasks',
  'comments',
  'automations',
  'meta',
] as const;
type ConfirmedStoreName = (typeof CONFIRMED_STORES)[number];
type ConfirmedTransaction = IDBPTransaction<ReplicaDB, ConfirmedStoreName[], 'readwrite'>;

const SCHEMA_VERSION_KEY = 'schema_version';
const NEEDS_BOOTSTRAP_KEY = 'needs_bootstrap';
const PROJECT_ORDER_KEY = 'project_order';
const SECTION_ORDER_KEY = 'section_order';
const TASK_ORDER_KEY = 'task_order';
const COMMENT_ORDER_KEY = 'comment_order';
const LAST_SEEN_SEQ_KEY = 'last_seen_seq';

export async function openReplicaDatabase(
  name: string,
  onBlocked?: () => void,
): Promise<IDBPDatabase<ReplicaDB>> {
  let database: IDBPDatabase<ReplicaDB> | undefined;
  let wasBlocked = false;

  try {
    const opened = await openDB<ReplicaDB>(name, REPLICA_DATABASE_VERSION, {
      upgrade(db, oldVersion, _newVersion, transaction) {
        if (!db.objectStoreNames.contains('projects')) {
          db.createObjectStore('projects', { keyPath: 'id' });
        }
        if (!db.objectStoreNames.contains('sections')) {
          db.createObjectStore('sections', { keyPath: 'id' });
        }
        if (!db.objectStoreNames.contains('tasks')) {
          db.createObjectStore('tasks', { keyPath: 'id' });
        }
        if (!db.objectStoreNames.contains('comments')) {
          db.createObjectStore('comments', { keyPath: 'id' });
        }
        if (!db.objectStoreNames.contains('automations')) {
          db.createObjectStore('automations', { keyPath: 'id' });
        }
        if (!db.objectStoreNames.contains('outbox')) {
          const outbox = db.createObjectStore('outbox', {
            autoIncrement: true,
            keyPath: 'seq',
          });
          outbox.createIndex('operation_id', 'operation.operation_id', { unique: true });
        }
        if (!db.objectStoreNames.contains('meta')) {
          db.createObjectStore('meta');
        }
        if (oldVersion > 0 && oldVersion < 3 && db.objectStoreNames.contains('outbox')) {
          // queued note operations cannot be sent after the comment rename, so drop them once
          void transaction.objectStore('outbox').clear();
        }
      },
      blocked() {
        wasBlocked = true;
        onBlocked?.();
      },
      blocking() {
        database?.close();
      },
    });
    database = opened;
    return opened;
  } catch (error) {
    throw replicaError(
      wasBlocked ? 'blocked' : 'storage',
      wasBlocked
        ? `opening replica database "${name}" was blocked`
        : `failed to open replica database "${name}"`,
      error,
    );
  }
}

export async function loadConfirmed(db: IDBPDatabase<ReplicaDB>): Promise<LoadedConfirmed> {
  try {
    const tx = db.transaction([...CONFIRMED_STORES], 'readonly');
    const meta = tx.objectStore('meta');
    const [
      projects,
      sections,
      tasks,
      comments,
      automations,
      schemaVersion,
      projectOrder,
      sectionOrder,
      taskOrder,
      commentOrder,
      needsBootstrap,
      lastSeenSeq,
    ] = await Promise.all([
      tx.objectStore('projects').getAll(),
      tx.objectStore('sections').getAll(),
      tx.objectStore('tasks').getAll(),
      tx.objectStore('comments').getAll(),
      tx.objectStore('automations').getAll(),
      meta.get(SCHEMA_VERSION_KEY),
      meta.get(PROJECT_ORDER_KEY),
      meta.get(SECTION_ORDER_KEY),
      meta.get(TASK_ORDER_KEY),
      meta.get(COMMENT_ORDER_KEY),
      meta.get(NEEDS_BOOTSTRAP_KEY),
      meta.get(LAST_SEEN_SEQ_KEY),
    ]);
    await tx.done;

    if (schemaVersion !== CONFIRMED_SCHEMA_VERSION) {
      // a stale confirmed schema is discarded, but the outbox is left untouched so pending
      // operations can still replay once the client re-bootstraps.
      const migration = db.transaction([...CONFIRMED_STORES], 'readwrite');
      const migrationMeta = migration.objectStore('meta');
      await Promise.all([
        migration.objectStore('projects').clear(),
        migration.objectStore('sections').clear(),
        migration.objectStore('tasks').clear(),
        migration.objectStore('comments').clear(),
        migration.objectStore('automations').clear(),
      ]);
      await migrationMeta.put(CONFIRMED_SCHEMA_VERSION, SCHEMA_VERSION_KEY);
      await migrationMeta.put(true, NEEDS_BOOTSTRAP_KEY);
      await migrationMeta.put(0, LAST_SEEN_SEQ_KEY);
      await migration.done;

      return { state: { projects: [] }, needsBootstrap: true, lastSeenSeq: 0 };
    }

    return {
      state: buildClientState(
        projects,
        sections,
        tasks,
        comments,
        automations,
        projectOrder,
        sectionOrder,
        taskOrder,
        commentOrder,
      ),
      needsBootstrap: needsBootstrap === true,
      lastSeenSeq: typeof lastSeenSeq === 'number' ? lastSeenSeq : 0,
    };
  } catch (error) {
    throw storageError(error);
  }
}

export async function saveConfirmed(
  db: IDBPDatabase<ReplicaDB>,
  state: ClientState,
  lastSeenSeq?: number,
  options: { onlyIfNeeded?: boolean } = {},
): Promise<boolean> {
  try {
    const tx = db.transaction([...CONFIRMED_STORES], 'readwrite');

    if (options.onlyIfNeeded) {
      const needed = await tx.objectStore('meta').get(NEEDS_BOOTSTRAP_KEY);
      if (needed !== true) {
        await tx.done;
        return false;
      }
    }

    await writeConfirmed(tx, state, lastSeenSeq);
    await tx.done;
    return true;
  } catch (error) {
    throw storageError(error);
  }
}

async function writeConfirmed(
  tx: ConfirmedTransaction,
  state: ClientState,
  lastSeenSeq?: number,
): Promise<void> {
  const projects = tx.objectStore('projects');
  const sections = tx.objectStore('sections');
  const tasks = tx.objectStore('tasks');
  const comments = tx.objectStore('comments');
  const automations = tx.objectStore('automations');
  const meta = tx.objectStore('meta');

  await Promise.all([
    projects.clear(),
    sections.clear(),
    tasks.clear(),
    comments.clear(),
    automations.clear(),
  ]);

  const projectOrder: string[] = [];
  const sectionOrder: string[] = [];
  const taskOrder: string[] = [];
  const commentOrder: string[] = [];

  for (const snapshot of state.projects) {
    const {
      sections: snapshotSections,
      tasks: snapshotTasks,
      comments: snapshotComments,
      automations: snapshotAutomations,
      ...project
    } = snapshot;
    projectOrder.push(project.id);
    await projects.put(project);
    for (const section of snapshotSections) {
      sectionOrder.push(section.id);
      await sections.put(section);
    }
    for (const task of snapshotTasks) {
      taskOrder.push(task.id);
      await tasks.put(task);
    }
    for (const comment of snapshotComments) {
      commentOrder.push(comment.id);
      await comments.put(comment);
    }
    for (const automation of snapshotAutomations) {
      await automations.put(automation);
    }
  }

  const metaWrites = [
    meta.put(projectOrder, PROJECT_ORDER_KEY),
    meta.put(sectionOrder, SECTION_ORDER_KEY),
    meta.put(taskOrder, TASK_ORDER_KEY),
    meta.put(commentOrder, COMMENT_ORDER_KEY),
    meta.put(CONFIRMED_SCHEMA_VERSION, SCHEMA_VERSION_KEY),
    meta.put(false, NEEDS_BOOTSTRAP_KEY),
  ];
  if (lastSeenSeq !== undefined) {
    metaWrites.push(meta.put(lastSeenSeq, LAST_SEEN_SEQ_KEY));
  }
  await Promise.all(metaWrites);
}

export async function applyConfirmedChangesIfNewer(
  db: IDBPDatabase<ReplicaDB>,
  changes: readonly Change[],
  nextSeq: number,
): Promise<AppliedSyncResult> {
  try {
    const tx = db.transaction([...CONFIRMED_STORES], 'readwrite');
    const meta = tx.objectStore('meta');
    const [
      projects,
      sections,
      tasks,
      comments,
      automations,
      projectOrder,
      sectionOrder,
      taskOrder,
      commentOrder,
      lastSeenSeq,
    ] = await Promise.all([
      tx.objectStore('projects').getAll(),
      tx.objectStore('sections').getAll(),
      tx.objectStore('tasks').getAll(),
      tx.objectStore('comments').getAll(),
      tx.objectStore('automations').getAll(),
      meta.get(PROJECT_ORDER_KEY),
      meta.get(SECTION_ORDER_KEY),
      meta.get(TASK_ORDER_KEY),
      meta.get(COMMENT_ORDER_KEY),
      meta.get(LAST_SEEN_SEQ_KEY),
    ]);

    const state = buildClientState(
      projects,
      sections,
      tasks,
      comments,
      automations,
      projectOrder,
      sectionOrder,
      taskOrder,
      commentOrder,
    );
    const current = typeof lastSeenSeq === 'number' ? lastSeenSeq : 0;

    if (nextSeq <= current) {
      await tx.done;
      return { applied: false, state, lastSeenSeq: current };
    }

    const next = applyConfirmedChanges(
      state,
      changes.filter((change) => change.seq > current),
    );
    await writeConfirmed(tx, next, nextSeq);
    await tx.done;
    return { applied: true, state: next, lastSeenSeq: nextSeq };
  } catch (error) {
    throw storageError(error);
  }
}

export async function loadOutbox(db: IDBPDatabase<ReplicaDB>): Promise<Operation[]> {
  try {
    const tx = db.transaction('outbox', 'readonly');
    const entries = await tx.store.getAll();
    await tx.done;
    return entries.map((entry) => entry.operation);
  } catch (error) {
    throw storageError(error);
  }
}

export async function enqueueOperation(
  db: IDBPDatabase<ReplicaDB>,
  operation: Operation,
): Promise<void> {
  try {
    const tx = db.transaction('outbox', 'readwrite');
    await tx.store.add({ operation });
    await tx.done;
  } catch (error) {
    throw storageError(error);
  }
}

export async function deleteOutboxOperations(
  db: IDBPDatabase<ReplicaDB>,
  operationIds: readonly string[],
): Promise<void> {
  try {
    const tx = db.transaction('outbox', 'readwrite');
    const index = tx.store.index('operation_id');
    for (const operationId of operationIds) {
      const key = await index.getKey(operationId);
      if (key !== undefined) {
        await tx.store.delete(key);
      }
    }
    await tx.done;
  } catch (error) {
    throw storageError(error);
  }
}

function buildClientState(
  projects: readonly Project[],
  sections: readonly Section[],
  tasks: readonly Task[],
  comments: readonly Comment[],
  automations: readonly Automation[],
  projectOrder: unknown,
  sectionOrder: unknown,
  taskOrder: unknown,
  commentOrder: unknown,
): ClientState {
  return {
    projects: buildSnapshots(
      orderById(projects, asStringArray(projectOrder)),
      orderById(sections, asStringArray(sectionOrder)),
      orderById(tasks, asStringArray(taskOrder)),
      orderById(comments, asStringArray(commentOrder)),
      automations,
    ),
  };
}

function buildSnapshots(
  projects: readonly Project[],
  sections: readonly Section[],
  tasks: readonly Task[],
  comments: readonly Comment[],
  automations: readonly Automation[],
): ProjectSnapshot[] {
  const snapshots = new Map<string, ProjectSnapshot>();
  for (const project of projects) {
    const snapshot: ProjectSnapshot = {
      ...project,
      sections: [],
      tasks: [],
      comments: [],
      automations: [],
    };
    snapshots.set(project.id, snapshot);
  }

  for (const section of sections) {
    snapshots.get(section.project_id)?.sections.push(section);
  }

  const tasksById = new Map<string, Task>();
  for (const task of tasks) {
    tasksById.set(task.id, task);
    snapshots.get(task.project_id)?.tasks.push(task);
  }

  for (const comment of comments) {
    const task = tasksById.get(comment.task_id);
    if (task !== undefined) {
      snapshots.get(task.project_id)?.comments.push(comment);
    }
  }

  for (const automation of automations) {
    snapshots.get(automation.project_id)?.automations.push(automation);
  }

  return [...snapshots.values()];
}

function orderById<T extends { id: string }>(items: readonly T[], order: readonly string[]): T[] {
  const remaining = new Map(items.map((item): [string, T] => [item.id, item]));
  const ordered: T[] = [];

  for (const id of order) {
    const item = remaining.get(id);
    if (item !== undefined) {
      ordered.push(item);
      remaining.delete(id);
    }
  }

  for (const item of items) {
    if (remaining.delete(item.id)) {
      ordered.push(item);
    }
  }

  return ordered;
}

function asStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((item): item is string => typeof item === 'string');
}

function storageError(error: unknown): ReplicaError {
  if (isReplicaError(error)) {
    return error;
  }

  const name =
    // DOMException and Error subclasses carry name, but a guarded object has no known shape
    typeof error === 'object' && error !== null ? (error as { name?: unknown }).name : undefined;

  if (name === 'QuotaExceededError') {
    return replicaError('quota', 'the replica database ran out of storage space', error);
  }

  return replicaError('storage', 'the replica database operation failed', error);
}
