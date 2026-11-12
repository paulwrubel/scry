import { keyBetween } from '@/contract/positions';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { Note } from '@/generated/Note';
import type { Operation } from '@/generated/Operation';
import type { Priority } from '@/generated/Priority';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import type { TaskState } from '@/generated/TaskState';
import { newId } from '@/utils/uuid';

export type TaskFieldPatch = {
  title: string;
  description: string;
  priority: Priority;
  tags: string[];
};

export function nextTaskPosition(tasks: readonly Task[], sectionId: string | null): string {
  const last = tasks
    .filter((task) => task.section_id === sectionId)
    .reduce<string | null>(
      (max, task) => (max === null || task.position > max ? task.position : max),
      null,
    );

  return keyBetween(last, null);
}

export function createTaskOperation(
  project: ProjectSnapshot,
  sectionId: string | null,
  title: string,
  externalIdentity: ExternalIdentity | null = null,
): Operation {
  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'create_task',
    id: newId(),
    project_id: project.id,
    title,
    description: null,
    priority: 'medium',
    section_id: sectionId,
    position: nextTaskPosition(project.tasks, sectionId),
    state: 'open',
    tags: [],
    created_at: new Date().toISOString(),
    external_identity: externalIdentity,
    external_metadata: null,
  };
}

export function normalizeTags(tags: string[]): string[] {
  return [
    ...new Set(tags.map((tag) => tag.trim().toLowerCase()).filter((tag) => tag.length > 0)),
  ].sort();
}

export function updateTaskOperation(task: Task, patch: TaskFieldPatch): Operation | null {
  const title = patch.title.trim();
  if (title.length === 0) {
    return null;
  }

  const description = patch.description.trim();
  const nextDescription = description.length === 0 ? null : description;
  const tags = normalizeTags(patch.tags);

  const titleChanged = title !== task.title;
  const descriptionChanged = nextDescription !== task.description;
  const priorityChanged = patch.priority !== task.priority;
  const tagsChanged = !sameTags(tags, task.tags);

  if (!titleChanged && !descriptionChanged && !priorityChanged && !tagsChanged) {
    return null;
  }

  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'update_task',
    id: task.id,
    title: titleChanged ? title : null,
    description: descriptionChanged ? nextDescription : undefined,
    priority: priorityChanged ? patch.priority : null,
    tags: tagsChanged ? tags : null,
    state: null,
  };
}

export function externalIdentityOperation(
  task: Task,
  identity: ExternalIdentity | null,
): Operation | null {
  if (sameIdentity(task.external_identity, identity)) {
    return null;
  }

  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'update_task',
    id: task.id,
    title: null,
    description: undefined,
    priority: null,
    tags: null,
    state: null,
    external_identity: identity,
    external_metadata: null,
  };
}

export function moveTaskOperation(
  task: Task,
  sectionId: string | null,
  position: string,
): Operation | null {
  if (task.section_id === sectionId && task.position === position) {
    return null;
  }

  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'move_task',
    id: task.id,
    section_id: sectionId,
    position,
  };
}

export function setTaskStateOperation(task: Task, state: TaskState): Operation | null {
  if (task.state === state) {
    return null;
  }

  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'update_task',
    id: task.id,
    title: null,
    description: undefined,
    priority: null,
    tags: null,
    state,
  };
}

export function deleteTaskOperation(task: Task): Operation {
  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'delete_task',
    id: task.id,
  };
}

export function createNoteOperation(task: Task, contents: string): Operation | null {
  const text = contents.trim();
  if (text.length === 0) {
    return null;
  }

  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'create_note',
    id: newId(),
    task_id: task.id,
    contents: text,
    created_at: new Date().toISOString(),
  };
}

export function updateNoteOperation(note: Note, contents: string): Operation | null {
  const text = contents.trim();
  if (text.length === 0 || text === note.contents) {
    return null;
  }

  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'update_note',
    id: note.id,
    contents: text,
  };
}

export function deleteNoteOperation(note: Note): Operation {
  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'delete_note',
    id: note.id,
  };
}

function sameTags(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((tag, index) => tag === right[index]);
}

function sameIdentity(left: ExternalIdentity | null, right: ExternalIdentity | null): boolean {
  if (left === null || right === null) {
    return left === right;
  }
  return (
    left.provider === right.provider &&
    left.owner === right.owner &&
    left.repo === right.repo &&
    left.number === right.number
  );
}
