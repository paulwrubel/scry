import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import type { Replica, ReplicaSnapshot } from '@/utils/replica';

export const CREATED_AT = '2026-01-01T00:00:00Z';

export function makeProject(overrides: Partial<ProjectSnapshot> = {}): ProjectSnapshot {
  return {
    id: 'project-1',
    name: 'Project',
    description: null,
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
    sections: [],
    tasks: [],
    comments: [],
    automations: [],
    ...overrides,
  };
}

export function makeSection(overrides: Partial<Section> = {}): Section {
  return {
    id: 'section-1',
    project_id: 'project-1',
    name: 'Section',
    position: 'a0',
    color: null,
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
    ...overrides,
  };
}

export function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 'task-1',
    project_id: 'project-1',
    title: 'Task title',
    description: null,
    priority: 'normal',
    section_id: null,
    position: 'a0',
    state: 'open',
    tags: [],
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
    external_identity: null,
    external_metadata: null,
    ...overrides,
  };
}

export type ReplicaSnapshotOverrides = Partial<
  Pick<ReplicaSnapshot, 'outbox' | 'needsBootstrap' | 'persisted' | 'lastSeenSeq'>
>;

export function makeReplicaSnapshot(
  projects: ProjectSnapshot[],
  overrides: ReplicaSnapshotOverrides = {},
): ReplicaSnapshot {
  return {
    confirmed: { projects },
    visible: { projects },
    outbox: [],
    needsBootstrap: false,
    persisted: true,
    lastSeenSeq: 0,
    ...overrides,
  };
}

export type ReplicaMockOptions = {
  enqueue?: (operation: Operation) => Promise<void>;
  reload?: () => Promise<void>;
  close?: () => void;
};

/** A complete Replica whose storage methods are no-ops, for tests that only need a snapshot. */
export function createReplicaMock(
  snapshot: ReplicaSnapshot,
  options: ReplicaMockOptions = {},
): Replica {
  const {
    enqueue = async () => undefined,
    reload = async () => undefined,
    close = () => undefined,
  } = options;

  return {
    confirmed: snapshot.confirmed,
    visible: snapshot.visible,
    outbox: snapshot.outbox,
    needsBootstrap: snapshot.needsBootstrap,
    persisted: snapshot.persisted,
    lastSeenSeq: snapshot.lastSeenSeq,
    getSnapshot: () => snapshot,
    subscribe: () => () => undefined,
    bootstrap: async () => undefined,
    applySyncResult: async () => ({ applied: false }),
    enqueue,
    acknowledge: async () => undefined,
    reload,
    close,
  };
}
