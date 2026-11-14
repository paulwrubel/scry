import type { Automation } from '@/generated/Automation';
import type { AutomationAction } from '@/generated/AutomationAction';
import type { Change } from '@/generated/Change';
import type { Color } from '@/generated/Color';
import type { Comment } from '@/generated/Comment';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { ExternalMetadata } from '@/generated/ExternalMetadata';
import type { Priority } from '@/generated/Priority';
import type { Project } from '@/generated/Project';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import type { TaskState } from '@/generated/TaskState';
import {
  findAutomation,
  findComment,
  findProject,
  findProjectForTask,
  findSection,
  findTask,
  removeAutomation,
  removeComment,
  removeSection,
  removeTask,
  setProject,
  upsertAutomation,
  upsertComment,
  upsertSection,
  upsertTask,
} from './state';
import type { ClientState } from './types';

export function applyChange(state: ClientState, change: Change): void {
  switch (change.entity_type) {
    case 'project':
      applyProjectChange(state, change);
      return;
    case 'section':
      applySectionChange(state, change);
      return;
    case 'task':
      applyTaskChange(state, change);
      return;
    case 'comment':
      applyCommentChange(state, change);
      return;
    case 'automation':
      applyAutomationChange(state, change);
      return;
  }
}

export function applyProjectChange(state: ClientState, change: Change): void {
  switch (change.kind) {
    case 'create': {
      // a create change carries the full serialized project, so the untyped field map matches the generated type
      const project = change.field_values as Project;
      const existing = findProject(state, project.id);
      if (existing) {
        setProject(existing, project);
      } else {
        state.projects.push({ ...project, sections: [], tasks: [], comments: [], automations: [] });
      }
      return;
    }
    case 'update': {
      const snapshot = findProject(state, change.entity_id);
      if (snapshot) {
        applyProjectValues(snapshot, change.field_values);
      }
      return;
    }
    case 'delete': {
      state.projects = state.projects.filter((snapshot) => snapshot.id !== change.entity_id);
      return;
    }
  }
}

export function applySectionChange(state: ClientState, change: Change): void {
  switch (change.kind) {
    case 'create': {
      // a create change carries the full serialized section, so the untyped field map matches the generated type
      const section = change.field_values as Section;
      const snapshot = findProject(state, section.project_id);
      if (snapshot) {
        upsertSection(snapshot, section);
      }
      return;
    }
    case 'update': {
      const section = findSection(state, change.entity_id);
      if (section) {
        applySectionValues(section, change.field_values);
      }
      return;
    }
    case 'delete': {
      removeSection(state, change.entity_id);
      return;
    }
  }
}

export function applyTaskChange(state: ClientState, change: Change): void {
  switch (change.kind) {
    case 'create': {
      // a create change carries the full serialized task, so the untyped field map matches the generated type
      const task = change.field_values as Task;
      const snapshot = findProject(state, task.project_id);
      if (snapshot) {
        upsertTask(snapshot, task);
      }
      return;
    }
    case 'update': {
      const task = findTask(state, change.entity_id);
      if (task) {
        applyTaskValues(task, change.field_values);
      }
      return;
    }
    case 'delete': {
      removeTask(state, change.entity_id);
      return;
    }
  }
}

export function applyCommentChange(state: ClientState, change: Change): void {
  switch (change.kind) {
    case 'create': {
      // a create change carries the full serialized comment, so the untyped field map matches the generated type
      const comment = change.field_values as Comment;
      const snapshot = findProjectForTask(state, comment.task_id);
      if (snapshot) {
        upsertComment(snapshot, comment);
      }
      return;
    }
    case 'update': {
      const comment = findComment(state, change.entity_id);
      if (comment) {
        if (typeof change.field_values.contents === 'string') {
          comment.contents = change.field_values.contents;
        }
        if (typeof change.field_values.updated_at === 'string') {
          comment.updated_at = change.field_values.updated_at;
        }
      }
      return;
    }
    case 'delete': {
      removeComment(state, change.entity_id);
      return;
    }
  }
}

export function applyAutomationChange(state: ClientState, change: Change): void {
  switch (change.kind) {
    case 'create': {
      // a create change carries the full serialized automation, so the untyped field map matches the generated type
      const automation = change.field_values as Automation;
      const snapshot = findProject(state, automation.project_id);
      if (snapshot) {
        upsertAutomation(snapshot, automation);
      }
      return;
    }
    case 'update': {
      const automation = findAutomation(state, change.entity_id);
      if (automation) {
        if (Array.isArray(change.field_values.actions)) {
          // the Array.isArray guard proves this is an array of serialized actions
          automation.actions = change.field_values.actions as AutomationAction[];
        }
        if (typeof change.field_values.updated_at === 'string') {
          automation.updated_at = change.field_values.updated_at;
        }
      }
      return;
    }
    case 'delete': {
      removeAutomation(state, change.entity_id);
      return;
    }
  }
}

export function applyProjectValues(
  snapshot: ProjectSnapshot,
  values: Record<string, unknown>,
): void {
  if (typeof values.name === 'string') {
    snapshot.name = values.name;
  }
  if ('description' in values) {
    snapshot.description = typeof values.description === 'string' ? values.description : null;
  }
  if (typeof values.updated_at === 'string') {
    snapshot.updated_at = values.updated_at;
  }
}

export function applySectionValues(section: Section, values: Record<string, unknown>): void {
  if (typeof values.name === 'string') {
    section.name = values.name;
  }
  if ('color' in values) {
    // the typeof guard proves a string, and the server only emits valid Color literals
    section.color = typeof values.color === 'string' ? (values.color as Color) : null;
  }
  if (typeof values.position === 'string') {
    section.position = values.position;
  }
  if (typeof values.updated_at === 'string') {
    section.updated_at = values.updated_at;
  }
}

export function applyTaskValues(task: Task, values: Record<string, unknown>): void {
  if (typeof values.title === 'string') {
    task.title = values.title;
  }
  if ('description' in values) {
    task.description = typeof values.description === 'string' ? values.description : null;
  }
  if (typeof values.priority === 'string') {
    // the typeof guard proves a string, and the server only emits valid Priority literals
    task.priority = values.priority as Priority;
  }
  if (Array.isArray(values.tags)) {
    task.tags = values.tags.filter((tag): tag is string => typeof tag === 'string');
  }
  if (typeof values.state === 'string') {
    // the typeof guard proves a string, and the server only emits valid TaskState literals
    task.state = values.state as TaskState;
  }
  if ('section_id' in values) {
    task.section_id = typeof values.section_id === 'string' ? values.section_id : null;
  }
  if (typeof values.position === 'string') {
    task.position = values.position;
  }
  if ('external_identity' in values) {
    // change field values are serialized Task fields, so their JSON shape
    // matches the generated types even though the map is untyped
    task.external_identity = values.external_identity as ExternalIdentity | null;
  }
  if ('external_metadata' in values) {
    // change field values are serialized Task fields, so their JSON shape
    // matches the generated types even though the map is untyped
    task.external_metadata = values.external_metadata as ExternalMetadata | null;
  }
  if (typeof values.updated_at === 'string') {
    task.updated_at = values.updated_at;
  }
}
