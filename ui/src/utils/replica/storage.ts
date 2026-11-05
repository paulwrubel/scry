import type { DBSchema, IDBPDatabase } from 'idb';
import { openDB } from 'idb';
import type { ClientState } from '@/contract/op-replay';
import type { Note } from '@/generated/Note';
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
  notes: { key: string; value: Note };
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
};

const SCHEMA_VERSION_KEY = 'schema_version';
const NEEDS_BOOTSTRAP_KEY = 'needs_bootstrap';
const PROJECT_ORDER_KEY = 'project_order';
const SECTION_ORDER_KEY = 'section_order';
const TASK_ORDER_KEY = 'task_order';
const NOTE_ORDER_KEY = 'note_order';

export async function openReplicaDatabase(
  name: string,
  onBlocked?: () => void,
): Promise<IDBPDatabase<ReplicaDB>> {
  let database: IDBPDatabase<ReplicaDB> | undefined;
  let wasBlocked = false;

  try {
    const opened = await openDB<ReplicaDB>(name, REPLICA_DATABASE_VERSION, {
      upgrade(db) {
        if (!db.objectStoreNames.contains('projects')) {
          db.createObjectStore('projects', { keyPath: 'id' });
        }
        if (!db.objectStoreNames.contains('sections')) {
          db.createObjectStore('sections', { keyPath: 'id' });
        }
        if (!db.objectStoreNames.contains('tasks')) {
          db.createObjectStore('tasks', { keyPath: 'id' });
        }
        if (!db.objectStoreNames.contains('notes')) {
          db.createObjectStore('notes', { keyPath: 'id' });
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
    const tx = db.transaction(['projects', 'sections', 'tasks', 'notes', 'meta'], 'readonly');
    const meta = tx.objectStore('meta');
    const [
      projects,
      sections,
      tasks,
      notes,
      schemaVersion,
      projectOrder,
      sectionOrder,
      taskOrder,
      noteOrder,
      needsBootstrap,
    ] = await Promise.all([
      tx.objectStore('projects').getAll(),
      tx.objectStore('sections').getAll(),
      tx.objectStore('tasks').getAll(),
      tx.objectStore('notes').getAll(),
      meta.get(SCHEMA_VERSION_KEY),
      meta.get(PROJECT_ORDER_KEY),
      meta.get(SECTION_ORDER_KEY),
      meta.get(TASK_ORDER_KEY),
      meta.get(NOTE_ORDER_KEY),
      meta.get(NEEDS_BOOTSTRAP_KEY),
    ]);
    await tx.done;

    if (schemaVersion !== CONFIRMED_SCHEMA_VERSION) {
      // a stale confirmed schema is discarded, but the outbox is left untouched so pending
      // operations can still replay once the client re-bootstraps.
      const migration = db.transaction(
        ['projects', 'sections', 'tasks', 'notes', 'meta'],
        'readwrite',
      );
      const migrationMeta = migration.objectStore('meta');
      await Promise.all([
        migration.objectStore('projects').clear(),
        migration.objectStore('sections').clear(),
        migration.objectStore('tasks').clear(),
        migration.objectStore('notes').clear(),
      ]);
      await migrationMeta.put(CONFIRMED_SCHEMA_VERSION, SCHEMA_VERSION_KEY);
      await migrationMeta.put(true, NEEDS_BOOTSTRAP_KEY);
      await migration.done;

      return { state: { projects: [] }, needsBootstrap: true };
    }

    return {
      state: {
        projects: buildSnapshots(
          orderById(projects, asStringArray(projectOrder)),
          orderById(sections, asStringArray(sectionOrder)),
          orderById(tasks, asStringArray(taskOrder)),
          orderById(notes, asStringArray(noteOrder)),
        ),
      },
      needsBootstrap: needsBootstrap === true,
    };
  } catch (error) {
    throw storageError(error);
  }
}

export async function saveConfirmed(
  db: IDBPDatabase<ReplicaDB>,
  state: ClientState,
): Promise<void> {
  try {
    const tx = db.transaction(['projects', 'sections', 'tasks', 'notes', 'meta'], 'readwrite');
    const projects = tx.objectStore('projects');
    const sections = tx.objectStore('sections');
    const tasks = tx.objectStore('tasks');
    const notes = tx.objectStore('notes');
    const meta = tx.objectStore('meta');

    await Promise.all([projects.clear(), sections.clear(), tasks.clear(), notes.clear()]);

    const projectOrder: string[] = [];
    const sectionOrder: string[] = [];
    const taskOrder: string[] = [];
    const noteOrder: string[] = [];

    for (const snapshot of state.projects) {
      const {
        sections: snapshotSections,
        tasks: snapshotTasks,
        notes: snapshotNotes,
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
      for (const note of snapshotNotes) {
        noteOrder.push(note.id);
        await notes.put(note);
      }
    }

    await Promise.all([
      meta.put(projectOrder, PROJECT_ORDER_KEY),
      meta.put(sectionOrder, SECTION_ORDER_KEY),
      meta.put(taskOrder, TASK_ORDER_KEY),
      meta.put(noteOrder, NOTE_ORDER_KEY),
      meta.put(CONFIRMED_SCHEMA_VERSION, SCHEMA_VERSION_KEY),
      meta.put(false, NEEDS_BOOTSTRAP_KEY),
    ]);
    await tx.done;
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

function buildSnapshots(
  projects: readonly Project[],
  sections: readonly Section[],
  tasks: readonly Task[],
  notes: readonly Note[],
): ProjectSnapshot[] {
  const snapshots = new Map<string, ProjectSnapshot>();
  for (const project of projects) {
    const snapshot: ProjectSnapshot = {
      ...project,
      sections: [],
      tasks: [],
      notes: [],
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

  for (const note of notes) {
    const task = tasksById.get(note.task_id);
    if (task !== undefined) {
      snapshots.get(task.project_id)?.notes.push(note);
    }
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
    typeof error === 'object' && error !== null ? (error as { name?: unknown }).name : undefined;

  if (name === 'QuotaExceededError') {
    return replicaError('quota', 'the replica database ran out of storage space', error);
  }

  return replicaError('storage', 'the replica database operation failed', error);
}
