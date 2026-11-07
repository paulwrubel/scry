import { describe, expect, it } from 'vitest';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import {
  createProjectOperation,
  deleteProjectOperation,
  updateProjectOperation,
} from './projectOperations';

function project(overrides: Partial<ProjectSnapshot> = {}): ProjectSnapshot {
  return {
    id: 'project-1',
    name: 'Project',
    description: 'Details',
    task_sorting_mode: 'manual',
    show_priority: false,
    created_at: '2026-01-01T00:00:00Z',
    sections: [],
    tasks: [],
    notes: [],
    ...overrides,
  };
}

const BASE = {
  name: 'Project',
  description: 'Details',
  task_sorting_mode: 'manual' as const,
  show_priority: false,
};

describe('updateProjectOperation', () => {
  it('returns null for a no-op', () => {
    expect(updateProjectOperation(project(), BASE)).toBeNull();
  });

  it('sends only the changed name', () => {
    const operation = updateProjectOperation(project(), { ...BASE, name: 'Renamed' });

    expect(operation).toMatchObject({
      kind: 'update_project',
      id: 'project-1',
      name: 'Renamed',
      task_sorting_mode: null,
      show_priority: null,
    });
    if (operation === null || operation.kind !== 'update_project') {
      throw new Error('expected an update_project operation');
    }
    expect(operation.description).toBeUndefined();
  });

  it('clears an emptied description', () => {
    expect(updateProjectOperation(project(), { ...BASE, description: '   ' })).toMatchObject({
      description: null,
    });
  });

  it('sends sorting and priority changes', () => {
    expect(
      updateProjectOperation(project(), {
        ...BASE,
        task_sorting_mode: 'priority',
        show_priority: true,
      }),
    ).toMatchObject({
      name: null,
      task_sorting_mode: 'priority',
      show_priority: true,
    });
  });

  it('rejects an empty name', () => {
    expect(updateProjectOperation(project(), { ...BASE, name: '  ' })).toBeNull();
  });
});

describe('deleteProjectOperation', () => {
  it('targets the project for deletion', () => {
    const operation = deleteProjectOperation(project());

    expect(operation).toMatchObject({
      kind: 'delete_project',
      id: 'project-1',
      operation_version: 1,
    });
    expect(operation.operation_id).toEqual(expect.any(String));
  });
});

describe('createProjectOperation', () => {
  it('creates a blank project with trimmed fields and defaults', () => {
    const operation = createProjectOperation({
      name: '  New project  ',
      description: '  Details  ',
    });

    if (operation.kind !== 'create_project') {
      throw new Error('expected a create_project operation');
    }

    expect(operation).toMatchObject({
      name: 'New project',
      description: 'Details',
      task_sorting_mode: 'manual',
      show_priority: false,
      operation_version: 1,
    });
    expect(operation.id).toEqual(expect.any(String));
    expect(operation.operation_id).not.toBe(operation.id);
    expect(operation.created_at).toBe(new Date(operation.created_at).toISOString());
  });

  it('stores an empty description as null', () => {
    const operation = createProjectOperation({ name: 'New project', description: '   ' });

    if (operation.kind !== 'create_project') {
      throw new Error('expected a create_project operation');
    }

    expect(operation.description).toBeNull();
  });
});
