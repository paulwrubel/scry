import 'fake-indexeddb/auto';
import { openDB } from 'idb';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { ClientState } from '@/contract/op-replay';
import type { Note } from '@/generated/Note';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import { openReplica } from './replica';
import type { ReplicaDB } from './storage';
import { CONFIRMED_SCHEMA_VERSION, REPLICA_DATABASE_VERSION } from './types';

const PROJECT_ID = 'project-1';
const SECTION_A_ID = 'section-a';
const SECTION_B_ID = 'section-b';
const TASK_A_ID = 'task-a';
const TASK_B_ID = 'task-b';
const NOTE_ID = 'note-a';

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
    const { sections, tasks, notes, ...projectRecord } = project;

    expect(await db.get('projects', project.id)).toEqual(projectRecord);
    expect(await db.get('sections', SECTION_A_ID)).toEqual(sections[0]);
    expect(await db.get('sections', SECTION_B_ID)).toEqual(sections[1]);
    expect(await db.get('tasks', TASK_A_ID)).toEqual(tasks[0]);
    expect(await db.get('tasks', TASK_B_ID)).toEqual(tasks[1]);
    expect(await db.get('notes', NOTE_ID)).toEqual(notes[0]);
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
      },
    ],
  };
}
