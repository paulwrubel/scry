import { describe, expect, it } from 'vitest';
import type { Change } from '../generated/Change';
import type { Note } from '../generated/Note';
import type { Operation } from '../generated/Operation';
import type { ProjectSnapshot } from '../generated/ProjectSnapshot';
import type { Section } from '../generated/Section';
import type { Task } from '../generated/Task';
import type { ClientState } from './op-replay';
import { applyChanges, replayOperations, visibleState } from './op-replay';

const PROJECT_ID = '11111111-1111-4111-8111-111111111111';
const SECTION_ID = '22222222-2222-4222-8222-222222222222';
const TASK_ID = '44444444-4444-4444-8444-444444444444';
const NOTE_ID = '66666666-6666-4666-8666-666666666666';
const CREATED_AT = '2026-01-01T00:00:00Z';

const section: Section = {
  id: SECTION_ID,
  project_id: PROJECT_ID,
  name: 'backlog',
  position: 'a0',
  color: null,
  created_at: CREATED_AT,
};

const note: Note = {
  id: NOTE_ID,
  task_id: TASK_ID,
  contents: 'note',
  created_at: CREATED_AT,
};

function task(title: string): Task {
  return {
    id: TASK_ID,
    project_id: PROJECT_ID,
    title,
    description: null,
    priority: 'medium',
    section_id: SECTION_ID,
    position: 'a0',
    state: 'open',
    tags: [],
    created_at: CREATED_AT,
  };
}

function snapshot(title: string): ProjectSnapshot {
  return {
    id: PROJECT_ID,
    name: 'project',
    description: null,
    task_sorting_mode: 'manual',
    show_priority: true,
    created_at: CREATED_AT,
    sections: [section],
    tasks: [task(title)],
    notes: [note],
  };
}

function state(): ClientState {
  return { projects: [snapshot('original')] };
}

function updateTask(title: string, operationId: string): Operation {
  return {
    operation_id: operationId,
    operation_version: 1,
    kind: 'update_task',
    id: TASK_ID,
    title,
    priority: null,
    tags: null,
    state: null,
  };
}

function serverChange(title: string): Change {
  return {
    seq: 2,
    entity_type: 'task',
    entity_id: TASK_ID,
    kind: 'update',
    operation_id: 'server-change',
    client_timestamp: null,
    field_values: { title },
    created_at: CREATED_AT,
  };
}

describe('op replay', () => {
  it('applyChanges applies an update_task change', () => {
    const result = applyChanges(state(), [serverChange('server')]);

    expect(result.projects[0].tasks[0].title).toBe('server');
  });

  it('replayOperations applies an update_task operation', () => {
    const result = replayOperations(state(), [updateTask('local', 'op-1')]);

    expect(result.projects[0].tasks[0].title).toBe('local');
  });

  it('visibleState keeps an unacknowledged local edit over a server change', () => {
    const result = visibleState(
      state(),
      [serverChange('server')],
      [],
      [updateTask('local', 'op-1')],
    );

    expect(result.projects[0].tasks[0].title).toBe('local');
  });

  it('visibleState drops an acknowledged local edit so the server change wins', () => {
    const result = visibleState(
      state(),
      [serverChange('server')],
      ['op-1'],
      [updateTask('local', 'op-1')],
    );

    expect(result.projects[0].tasks[0].title).toBe('server');
  });
});
