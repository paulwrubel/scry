import { describe, expect, it } from 'vitest';
import type { Automation } from '@/generated/Automation';
import type { AutomationAction } from '@/generated/AutomationAction';
import type { Change } from '@/generated/Change';
import type { Comment } from '@/generated/Comment';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import type { ClientState } from './index';
import { applyChanges, replayOperations, visibleState } from './index';

const PROJECT_ID = '11111111-1111-4111-8111-111111111111';
const SECTION_ID = '22222222-2222-4222-8222-222222222222';
const TASK_ID = '44444444-4444-4444-8444-444444444444';
const COMMENT_ID = '66666666-6666-4666-8666-666666666666';
const AUTOMATION_ID = '88888888-8888-4888-8888-888888888888';
const CREATED_AT = '2026-01-01T00:00:00Z';

const section: Section = {
  id: SECTION_ID,
  project_id: PROJECT_ID,
  name: 'backlog',
  position: 'a0',
  color: null,
  created_at: CREATED_AT,
  updated_at: CREATED_AT,
};

const comment: Comment = {
  id: COMMENT_ID,
  task_id: TASK_ID,
  contents: 'comment',
  created_at: CREATED_AT,
  updated_at: CREATED_AT,
};

function task(title: string): Task {
  return {
    id: TASK_ID,
    project_id: PROJECT_ID,
    title,
    description: null,
    priority: 'normal',
    section_id: SECTION_ID,
    position: 'a0',
    state: 'open',
    tags: [],
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
    external_identity: null,
    external_metadata: null,
  };
}

function snapshot(title: string): ProjectSnapshot {
  return {
    id: PROJECT_ID,
    name: 'project',
    description: null,
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
    sections: [section],
    tasks: [task(title)],
    comments: [comment],
    automations: [],
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

function serverSectionChange(position: string): Change {
  return {
    seq: 2,
    entity_type: 'section',
    entity_id: SECTION_ID,
    kind: 'update',
    operation_id: 'server-change',
    client_timestamp: null,
    field_values: { position },
    created_at: CREATED_AT,
  };
}

function moveSection(position: string, operationId: string): Operation {
  return {
    operation_id: operationId,
    operation_version: 1,
    kind: 'move_section',
    id: SECTION_ID,
    position,
  };
}

function deleteProject(operationId: string): Operation {
  return {
    operation_id: operationId,
    operation_version: 1,
    kind: 'delete_project',
    id: PROJECT_ID,
  };
}

function serverProjectDelete(): Change {
  return {
    seq: 2,
    entity_type: 'project',
    entity_id: PROJECT_ID,
    kind: 'delete',
    operation_id: 'server-change',
    client_timestamp: null,
    field_values: {},
    created_at: CREATED_AT,
  };
}

function automation(
  actions: AutomationAction[] = [{ kind: 'move_to_section', section_id: SECTION_ID }],
): Automation {
  return {
    id: AUTOMATION_ID,
    project_id: PROJECT_ID,
    trigger: { kind: 'task_enters_section', section_id: SECTION_ID },
    actions,
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
  };
}

function createAutomation(operationId: string): Operation {
  return {
    operation_id: operationId,
    operation_version: 1,
    kind: 'create_automation',
    id: AUTOMATION_ID,
    project_id: PROJECT_ID,
    trigger: { kind: 'task_enters_section', section_id: SECTION_ID },
    actions: [{ kind: 'move_to_section', section_id: SECTION_ID }],
    created_at: CREATED_AT,
  };
}

function updateAutomation(operationId: string): Operation {
  return {
    operation_id: operationId,
    operation_version: 1,
    kind: 'update_automation',
    id: AUTOMATION_ID,
    actions: [{ kind: 'set_priority', priority: 'elevated' }],
  };
}

function deleteAutomation(operationId: string): Operation {
  return {
    operation_id: operationId,
    operation_version: 1,
    kind: 'delete_automation',
    id: AUTOMATION_ID,
  };
}

function automationChange(kind: Change['kind'], fieldValues: Record<string, unknown>): Change {
  return {
    seq: 2,
    entity_type: 'automation',
    entity_id: AUTOMATION_ID,
    kind,
    operation_id: 'automation-change',
    client_timestamp: null,
    field_values: fieldValues,
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

  it('applyChanges applies a section position change', () => {
    const result = applyChanges(state(), [serverSectionChange('a1')]);

    expect(result.projects[0].sections[0].position).toBe('a1');
  });

  it('visibleState keeps the server position once a section move is acknowledged', () => {
    const result = visibleState(
      state(),
      [serverSectionChange('a1')],
      ['op-1'],
      [moveSection('a1', 'op-1')],
    );

    expect(result.projects[0].sections[0].position).toBe('a1');
  });

  it('replayOperations removes a project on delete_project', () => {
    const result = replayOperations(state(), [deleteProject('op-1')]);

    expect(result.projects).toHaveLength(0);
  });

  it('applyChanges removes a project on a project delete change', () => {
    const result = applyChanges(state(), [serverProjectDelete()]);

    expect(result.projects).toHaveLength(0);
  });

  it('replays a legacy create_project operation carrying removed fields', () => {
    const legacy = {
      operation_id: 'op-legacy',
      operation_version: 1,
      kind: 'create_project',
      id: PROJECT_ID,
      name: 'Legacy project',
      description: null,
      task_sorting_mode: 'priority',
      show_priority: true,
      created_at: CREATED_AT,
    } as Operation;

    const result = replayOperations({ projects: [] }, [legacy]);

    expect(result.projects).toHaveLength(1);
    expect(result.projects[0]).toMatchObject({
      id: PROJECT_ID,
      name: 'Legacy project',
      description: null,
    });
  });

  it('replays a legacy update_project operation carrying removed fields', () => {
    const legacy = {
      operation_id: 'op-legacy',
      operation_version: 1,
      kind: 'update_project',
      id: PROJECT_ID,
      name: 'Renamed',
      description: 'Updated',
      task_sorting_mode: 'priority',
      show_priority: false,
    } as Operation;

    const result = replayOperations(state(), [legacy]);

    expect(result.projects[0]).toMatchObject({
      name: 'Renamed',
      description: 'Updated',
    });
  });
});

describe('automation replay', () => {
  it('replayOperations creates an automation in the right project', () => {
    const result = replayOperations(state(), [createAutomation('op-create')]);

    expect(result.projects[0].automations).toEqual([automation()]);
  });

  it('replayOperations replaces an automation action list', () => {
    const confirmed = state();
    confirmed.projects[0].automations = [automation()];

    const result = replayOperations(confirmed, [updateAutomation('op-update')]);

    expect(result.projects[0].automations[0].actions).toEqual([
      { kind: 'set_priority', priority: 'elevated' },
    ]);
  });

  it('replayOperations removes an automation', () => {
    const confirmed = state();
    confirmed.projects[0].automations = [automation()];

    const result = replayOperations(confirmed, [deleteAutomation('op-delete')]);

    expect(result.projects[0].automations).toEqual([]);
  });

  it('applyChanges applies automation create, update, and delete', () => {
    const created = applyChanges(state(), [automationChange('create', automation())]);
    expect(created.projects[0].automations).toEqual([automation()]);

    const updated = applyChanges(created, [
      automationChange('update', { actions: [{ kind: 'open_task' }] }),
    ]);
    expect(updated.projects[0].automations[0].actions).toEqual([{ kind: 'open_task' }]);

    const deleted = applyChanges(updated, [automationChange('delete', {})]);
    expect(deleted.projects[0].automations).toEqual([]);
  });
});
