import { describe, expect, it } from 'vitest';
import type { Comment } from '@/generated/Comment';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { ExternalMetadata } from '@/generated/ExternalMetadata';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import {
  createCommentOperation,
  createTaskOperation,
  deleteCommentOperation,
  deleteTaskOperation,
  externalIdentityOperation,
  moveTaskOperation,
  nextTaskPosition,
  setTaskStateOperation,
  updateCommentOperation,
  updateTaskOperation,
} from './operations';

const PROJECT_ID = 'project-1';
const TASK_ID = 'task-1';
const CREATED_AT = '2026-01-01T00:00:00Z';

function projectSnapshot(overrides: Partial<ProjectSnapshot> = {}): ProjectSnapshot {
  return {
    id: PROJECT_ID,
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

function task(overrides: Partial<Task> & Pick<Task, 'id' | 'title'>): Task {
  return {
    project_id: PROJECT_ID,
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

function comment(overrides: Partial<Comment> & Pick<Comment, 'id' | 'contents'>): Comment {
  return {
    task_id: TASK_ID,
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
    ...overrides,
  };
}

describe('nextTaskPosition', () => {
  it('appends after the last task in the section', () => {
    const tasks = [
      task({ id: 'one', title: 'One', section_id: 'backlog', position: 'a0' }),
      task({ id: 'two', title: 'Two', section_id: 'backlog', position: 'a1' }),
    ];

    expect(nextTaskPosition(tasks, 'backlog')).toBe('a2');
  });

  it('ignores other sections and includes closed tasks', () => {
    const tasks = [
      task({ id: 'one', title: 'One', section_id: 'backlog', position: 'a0' }),
      task({
        id: 'closed',
        title: 'Closed',
        section_id: 'backlog',
        position: 'a2',
        state: 'closed',
      }),
      task({ id: 'other', title: 'Other', section_id: 'doing', position: 'a9' }),
    ];

    expect(nextTaskPosition(tasks, 'backlog')).toBe('a3');
  });

  it('starts at a0 for an empty section', () => {
    expect(nextTaskPosition([], 'backlog')).toBe('a0');
  });

  it('appends tasks without a section', () => {
    const tasks = [
      task({ id: 'one', title: 'One', position: 'a1' }),
      task({ id: 'two', title: 'Two', section_id: 'backlog', position: 'a9' }),
    ];

    expect(nextTaskPosition(tasks, null)).toBe('a2');
  });
});

describe('createTaskOperation', () => {
  it('builds an open task at the end of the section', () => {
    const project = projectSnapshot({
      tasks: [task({ id: 'one', title: 'One', section_id: 'backlog', position: 'a0' })],
    });

    const operation = createTaskOperation(project, 'backlog', 'New task');

    expect(operation).toMatchObject({
      operation_version: 1,
      kind: 'create_task',
      project_id: PROJECT_ID,
      title: 'New task',
      description: null,
      priority: 'normal',
      section_id: 'backlog',
      position: 'a1',
      state: 'open',
      tags: [],
    });
    expect(operation.kind === 'create_task' && operation.id).not.toBe(operation.operation_id);
    expect(operation.kind === 'create_task' && operation.created_at).toMatch(/Z$/);
  });

  it('includes the external identity when provided', () => {
    const identity: ExternalIdentity = {
      provider: 'github',
      owner: 'acme',
      repo: 'widgets',
      number: 123,
    };

    const operation = createTaskOperation(projectSnapshot(), null, 'New task', identity);

    expect(operation).toMatchObject({
      kind: 'create_task',
      external_identity: identity,
      external_metadata: null,
    });
  });
});

describe('updateTaskOperation', () => {
  const existing = task({
    id: TASK_ID,
    title: 'Title',
    description: 'Description',
    priority: 'normal',
    tags: ['one'],
  });

  it('returns null when nothing changed', () => {
    expect(
      updateTaskOperation(existing, {
        title: 'Title',
        description: 'Description',
        priority: 'normal',
        tags: ['one'],
      }),
    ).toBeNull();
  });

  it('returns null for an empty title', () => {
    expect(
      updateTaskOperation(existing, {
        title: '   ',
        description: 'Description',
        priority: 'normal',
        tags: ['one'],
      }),
    ).toBeNull();
  });

  it('sends only changed fields', () => {
    const operation = updateTaskOperation(existing, {
      title: 'New title',
      description: 'Description',
      priority: 'normal',
      tags: ['one'],
    });

    expect(operation).toMatchObject({
      kind: 'update_task',
      id: TASK_ID,
      title: 'New title',
      priority: null,
      tags: null,
      state: null,
    });
    expect(operation?.kind === 'update_task' && operation.description).toBeUndefined();
  });

  it('clears an emptied description', () => {
    const operation = updateTaskOperation(existing, {
      title: 'Title',
      description: '   ',
      priority: 'normal',
      tags: ['one'],
    });

    expect(operation).toMatchObject({ description: null, title: null, priority: null });
  });

  it('trims, lowercases, dedupes, and sorts tags', () => {
    const operation = updateTaskOperation(existing, {
      title: 'Title',
      description: 'Description',
      priority: 'normal',
      tags: [' Two ', 'three', '', 'two'],
    });

    expect(operation).toMatchObject({ tags: ['three', 'two'], title: null });
  });

  it('does not send locked title or description when only tags change', () => {
    const identity: ExternalIdentity = {
      provider: 'github',
      owner: 'acme',
      repo: 'widgets',
      number: 123,
    };
    const metadata: ExternalMetadata = {
      provider: 'github',
      title: 'Title',
      description: 'Body\n',
      kind: 'issue',
      state: 'open',
      reason: null,
      draft: null,
      updated_at: CREATED_AT,
      error: null,
    };
    const linked = task({
      id: TASK_ID,
      title: 'Title',
      description: 'Body\n',
      priority: 'normal',
      tags: ['one'],
      external_identity: identity,
      external_metadata: metadata,
    });

    const operation = updateTaskOperation(linked, {
      title: 'Title',
      description: 'Body\n',
      priority: 'normal',
      tags: ['one', 'two'],
    });

    expect(operation).toMatchObject({
      kind: 'update_task',
      id: TASK_ID,
      title: null,
      tags: ['one', 'two'],
      state: null,
    });
    expect(operation?.kind === 'update_task' && operation.description).toBeUndefined();
  });

  it('does not send a locked title when only tags change', () => {
    const identity: ExternalIdentity = {
      provider: 'github',
      owner: 'acme',
      repo: 'widgets',
      number: 123,
    };
    const metadata: ExternalMetadata = {
      provider: 'github',
      title: 'Title ',
      description: null,
      kind: 'issue',
      state: 'open',
      reason: null,
      draft: null,
      updated_at: CREATED_AT,
      error: null,
    };
    const linked = task({
      id: TASK_ID,
      title: 'Title ',
      priority: 'normal',
      tags: ['one'],
      external_identity: identity,
      external_metadata: metadata,
    });

    const operation = updateTaskOperation(linked, {
      title: 'Title ',
      description: '',
      priority: 'normal',
      tags: ['one', 'two'],
    });

    expect(operation).toMatchObject({
      kind: 'update_task',
      id: TASK_ID,
      title: null,
      tags: ['one', 'two'],
      state: null,
    });
  });
});

describe('externalIdentityOperation', () => {
  const identity: ExternalIdentity = {
    provider: 'github',
    owner: 'acme',
    repo: 'widgets',
    number: 123,
  };

  it('links an unlinked task', () => {
    const operation = externalIdentityOperation(task({ id: TASK_ID, title: 'Title' }), identity);

    expect(operation).toMatchObject({
      kind: 'update_task',
      id: TASK_ID,
      external_identity: identity,
      external_metadata: null,
    });
  });

  it('clears a linked task', () => {
    const linked = task({ id: TASK_ID, title: 'Title', external_identity: identity });

    expect(externalIdentityOperation(linked, null)).toMatchObject({
      kind: 'update_task',
      id: TASK_ID,
      external_identity: null,
      external_metadata: null,
    });
  });

  it('returns null when the identity is unchanged', () => {
    const linked = task({ id: TASK_ID, title: 'Title', external_identity: identity });

    expect(externalIdentityOperation(linked, { ...identity })).toBeNull();
  });

  it('returns null for an unlinked task with no identity', () => {
    expect(externalIdentityOperation(task({ id: TASK_ID, title: 'Title' }), null)).toBeNull();
  });
});

describe('moveTaskOperation', () => {
  it('returns null when the section and position are unchanged', () => {
    const existing = task({ id: TASK_ID, title: 'Title', section_id: 'backlog', position: 'a1' });

    expect(moveTaskOperation(existing, 'backlog', 'a1')).toBeNull();
  });

  it('reorders within a section', () => {
    const existing = task({ id: TASK_ID, title: 'Title', section_id: 'backlog', position: 'a1' });

    expect(moveTaskOperation(existing, 'backlog', 'a0V')).toMatchObject({
      kind: 'move_task',
      id: TASK_ID,
      section_id: 'backlog',
      position: 'a0V',
    });
  });

  it('moves to another section', () => {
    const existing = task({ id: TASK_ID, title: 'Title', section_id: 'backlog', position: 'a1' });

    expect(moveTaskOperation(existing, 'doing', 'a0')).toMatchObject({
      kind: 'move_task',
      id: TASK_ID,
      section_id: 'doing',
      position: 'a0',
    });
  });

  it('moves to No section', () => {
    const existing = task({ id: TASK_ID, title: 'Title', section_id: 'backlog', position: 'a1' });

    expect(moveTaskOperation(existing, null, 'a0')).toMatchObject({
      kind: 'move_task',
      section_id: null,
      position: 'a0',
    });
  });
});

describe('setTaskStateOperation', () => {
  it('returns null when the state is unchanged', () => {
    expect(setTaskStateOperation(task({ id: TASK_ID, title: 'Title' }), 'open')).toBeNull();
  });

  it('updates only the state', () => {
    expect(setTaskStateOperation(task({ id: TASK_ID, title: 'Title' }), 'closed')).toMatchObject({
      kind: 'update_task',
      id: TASK_ID,
      title: null,
      priority: null,
      tags: null,
      state: 'closed',
    });
  });
});

describe('deleteTaskOperation', () => {
  it('targets the task', () => {
    expect(deleteTaskOperation(task({ id: TASK_ID, title: 'Title' }))).toMatchObject({
      kind: 'delete_task',
      id: TASK_ID,
    });
  });
});

describe('comment operations', () => {
  const existingTask = task({ id: TASK_ID, title: 'Title' });

  it('creates a trimmed comment', () => {
    const operation = createCommentOperation(existingTask, '  comment text  ');

    expect(operation).toMatchObject({
      kind: 'create_comment',
      task_id: TASK_ID,
      contents: 'comment text',
    });
    expect(operation?.kind === 'create_comment' && operation?.id).not.toBe(operation?.operation_id);
  });

  it('does not create an empty comment', () => {
    expect(createCommentOperation(existingTask, '   ')).toBeNull();
  });

  it('updates a changed comment', () => {
    const existing = comment({ id: 'comment-1', contents: 'old' });

    expect(updateCommentOperation(existing, ' new ')).toMatchObject({
      kind: 'update_comment',
      id: 'comment-1',
      contents: 'new',
    });
  });

  it('does not update an unchanged or empty comment', () => {
    const existing = comment({ id: 'comment-1', contents: 'same' });

    expect(updateCommentOperation(existing, 'same')).toBeNull();
    expect(updateCommentOperation(existing, '  ')).toBeNull();
  });

  it('deletes a comment', () => {
    expect(deleteCommentOperation(comment({ id: 'comment-1', contents: 'text' }))).toMatchObject({
      kind: 'delete_comment',
      id: 'comment-1',
    });
  });
});
