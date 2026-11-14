import type { Automation } from '../generated/Automation';
import type { AutomationAction } from '../generated/AutomationAction';
import type { Change } from '../generated/Change';
import type { Color } from '../generated/Color';
import type { ExternalIdentity } from '../generated/ExternalIdentity';
import type { ExternalMetadata } from '../generated/ExternalMetadata';
import type { Comment } from '../generated/Comment';
import type { Operation } from '../generated/Operation';
import type { Priority } from '../generated/Priority';
import type { Project } from '../generated/Project';
import type { ProjectSnapshot } from '../generated/ProjectSnapshot';
import type { Section } from '../generated/Section';
import type { Task } from '../generated/Task';
import type { TaskState } from '../generated/TaskState';

export type ClientState = {
  projects: ProjectSnapshot[];
};

export function applyChanges(confirmed: ClientState, changes: readonly Change[]): ClientState {
  const state = structuredClone(confirmed);
  for (const change of changes) {
    applyChange(state, change);
  }
  return state;
}

export function replayOperations(
  confirmed: ClientState,
  operations: readonly Operation[],
): ClientState {
  const state = structuredClone(confirmed);
  for (const operation of operations) {
    applyOperation(state, operation);
  }
  return state;
}

export function visibleState(
  confirmed: ClientState,
  changes: readonly Change[],
  acknowledged: readonly string[],
  outbox: readonly Operation[],
): ClientState {
  const confirmedAfterPull = applyChanges(confirmed, changes);
  const acked = new Set(acknowledged);
  const remaining = outbox.filter((operation) => !acked.has(operation.operation_id));

  return replayOperations(confirmedAfterPull, remaining);
}

function applyOperation(state: ClientState, operation: Operation): void {
  switch (operation.kind) {
    case 'create_project': {
      const project: Project = {
        id: operation.id,
        name: operation.name,
        description: operation.description,
        created_at: operation.created_at,
        updated_at: operation.updated_at ?? operation.created_at,
      };
      const existing = findProject(state, operation.id);
      if (existing) {
        setProject(existing, project);
      } else {
        state.projects.push({ ...project, sections: [], tasks: [], comments: [], automations: [] });
      }
      return;
    }
    case 'update_project': {
      const snapshot = findProject(state, operation.id);
      if (!snapshot) {
        return;
      }
      if (operation.name !== undefined && operation.name !== null) {
        snapshot.name = operation.name;
      }
      if (operation.description !== undefined) {
        snapshot.description = operation.description;
      }
      if (operation.updated_at !== undefined && operation.updated_at !== null) {
        snapshot.updated_at = operation.updated_at;
      }
      return;
    }
    case 'delete_project': {
      state.projects = state.projects.filter((snapshot) => snapshot.id !== operation.id);
      return;
    }
    case 'create_section': {
      const section: Section = {
        id: operation.id,
        project_id: operation.project_id,
        name: operation.name,
        position: operation.position,
        color: operation.color,
        created_at: operation.created_at,
        updated_at: operation.updated_at ?? operation.created_at,
      };
      const snapshot = findProject(state, section.project_id);
      if (snapshot) {
        upsertSection(snapshot, section);
      }
      return;
    }
    case 'update_section': {
      const section = findSection(state, operation.id);
      if (!section) {
        return;
      }
      if (operation.name !== undefined && operation.name !== null) {
        section.name = operation.name;
      }
      if (operation.color !== undefined && operation.color !== null) {
        section.color = operation.color === 'clear' ? null : operation.color.set;
      }
      if (operation.updated_at !== undefined && operation.updated_at !== null) {
        section.updated_at = operation.updated_at;
      }
      return;
    }
    case 'move_section': {
      const section = findSection(state, operation.id);
      if (section) {
        section.position = operation.position;
        if (operation.updated_at !== undefined && operation.updated_at !== null) {
          section.updated_at = operation.updated_at;
        }
      }
      return;
    }
    case 'delete_section': {
      removeSection(state, operation.id);
      return;
    }
    case 'create_task': {
      const task: Task = {
        id: operation.id,
        project_id: operation.project_id,
        title: operation.title,
        description: operation.description,
        priority: operation.priority,
        section_id: operation.section_id,
        position: operation.position,
        state: operation.state,
        tags: operation.tags,
        created_at: operation.created_at,
        updated_at: operation.updated_at ?? operation.created_at,
        external_identity: operation.external_identity ?? null,
        external_metadata: operation.external_metadata ?? null,
      };
      const snapshot = findProject(state, task.project_id);
      if (snapshot) {
        upsertTask(snapshot, task);
      }
      return;
    }
    case 'update_task': {
      const task = findTask(state, operation.id);
      if (!task) {
        return;
      }
      if (operation.title !== undefined && operation.title !== null) {
        task.title = operation.title;
      }
      if (operation.description !== undefined) {
        task.description = operation.description;
      }
      if (operation.priority !== undefined && operation.priority !== null) {
        task.priority = operation.priority;
      }
      if (operation.tags !== undefined && operation.tags !== null) {
        task.tags = operation.tags;
      }
      if (operation.state !== undefined && operation.state !== null) {
        task.state = operation.state;
      }
      if (operation.external_identity !== undefined) {
        task.external_identity = operation.external_identity;
      }
      if (operation.external_metadata !== undefined) {
        task.external_metadata = operation.external_metadata;
      }
      if (operation.updated_at !== undefined && operation.updated_at !== null) {
        task.updated_at = operation.updated_at;
      }
      return;
    }
    case 'move_task': {
      const task = findTask(state, operation.id);
      if (task) {
        task.section_id = operation.section_id;
        task.position = operation.position;
        if (operation.updated_at !== undefined && operation.updated_at !== null) {
          task.updated_at = operation.updated_at;
        }
      }
      return;
    }
    case 'delete_task': {
      removeTask(state, operation.id);
      return;
    }
    case 'create_comment': {
      const comment: Comment = {
        id: operation.id,
        task_id: operation.task_id,
        contents: operation.contents,
        created_at: operation.created_at,
        updated_at: operation.updated_at ?? operation.created_at,
      };
      const snapshot = findProjectForTask(state, comment.task_id);
      if (snapshot) {
        upsertComment(snapshot, comment);
      }
      return;
    }
    case 'update_comment': {
      const comment = findComment(state, operation.id);
      if (comment) {
        comment.contents = operation.contents;
        if (operation.updated_at !== undefined && operation.updated_at !== null) {
          comment.updated_at = operation.updated_at;
        }
      }
      return;
    }
    case 'delete_comment': {
      removeComment(state, operation.id);
      return;
    }
    case 'create_automation': {
      const automation: Automation = {
        id: operation.id,
        project_id: operation.project_id,
        trigger: operation.trigger,
        actions: operation.actions,
        created_at: operation.created_at,
        updated_at: operation.updated_at ?? operation.created_at,
      };
      const snapshot = findProject(state, automation.project_id);
      if (snapshot) {
        upsertAutomation(snapshot, automation);
      }
      return;
    }
    case 'update_automation': {
      const automation = findAutomation(state, operation.id);
      if (automation) {
        automation.actions = operation.actions;
        if (operation.updated_at !== undefined && operation.updated_at !== null) {
          automation.updated_at = operation.updated_at;
        }
      }
      return;
    }
    case 'delete_automation': {
      removeAutomation(state, operation.id);
      return;
    }
  }
}

function applyChange(state: ClientState, change: Change): void {
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

function applyProjectChange(state: ClientState, change: Change): void {
  switch (change.kind) {
    case 'create': {
      const project = change.field_values as unknown as Project;
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

function applySectionChange(state: ClientState, change: Change): void {
  switch (change.kind) {
    case 'create': {
      const section = change.field_values as unknown as Section;
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

function applyTaskChange(state: ClientState, change: Change): void {
  switch (change.kind) {
    case 'create': {
      const task = change.field_values as unknown as Task;
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

function applyCommentChange(state: ClientState, change: Change): void {
  switch (change.kind) {
    case 'create': {
      const comment = change.field_values as unknown as Comment;
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

function applyAutomationChange(state: ClientState, change: Change): void {
  switch (change.kind) {
    case 'create': {
      const automation = change.field_values as unknown as Automation;
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

function applyProjectValues(snapshot: ProjectSnapshot, values: Record<string, unknown>): void {
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

function applySectionValues(section: Section, values: Record<string, unknown>): void {
  if (typeof values.name === 'string') {
    section.name = values.name;
  }
  if ('color' in values) {
    section.color = typeof values.color === 'string' ? (values.color as Color) : null;
  }
  if (typeof values.position === 'string') {
    section.position = values.position;
  }
  if (typeof values.updated_at === 'string') {
    section.updated_at = values.updated_at;
  }
}

function applyTaskValues(task: Task, values: Record<string, unknown>): void {
  if (typeof values.title === 'string') {
    task.title = values.title;
  }
  if ('description' in values) {
    task.description = typeof values.description === 'string' ? values.description : null;
  }
  if (typeof values.priority === 'string') {
    task.priority = values.priority as Priority;
  }
  if (Array.isArray(values.tags)) {
    task.tags = values.tags.filter((tag): tag is string => typeof tag === 'string');
  }
  if (typeof values.state === 'string') {
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

function setProject(snapshot: ProjectSnapshot, project: Project): void {
  snapshot.id = project.id;
  snapshot.name = project.name;
  snapshot.description = project.description;
  snapshot.created_at = project.created_at;
  snapshot.updated_at = project.updated_at;
}

function upsertSection(snapshot: ProjectSnapshot, section: Section): void {
  const index = snapshot.sections.findIndex((existing) => existing.id === section.id);
  if (index === -1) {
    snapshot.sections.push(section);
  } else {
    snapshot.sections[index] = section;
  }
}

function upsertTask(snapshot: ProjectSnapshot, task: Task): void {
  const index = snapshot.tasks.findIndex((existing) => existing.id === task.id);
  if (index === -1) {
    snapshot.tasks.push(task);
  } else {
    snapshot.tasks[index] = task;
  }
}

function upsertComment(snapshot: ProjectSnapshot, comment: Comment): void {
  const index = snapshot.comments.findIndex((existing) => existing.id === comment.id);
  if (index === -1) {
    snapshot.comments.push(comment);
  } else {
    snapshot.comments[index] = comment;
  }
}

function upsertAutomation(snapshot: ProjectSnapshot, automation: Automation): void {
  const index = snapshot.automations.findIndex((existing) => existing.id === automation.id);
  if (index === -1) {
    snapshot.automations.push(automation);
  } else {
    snapshot.automations[index] = automation;
  }
}

function findProject(state: ClientState, id: string): ProjectSnapshot | undefined {
  return state.projects.find((snapshot) => snapshot.id === id);
}

function findProjectForTask(state: ClientState, taskId: string): ProjectSnapshot | undefined {
  return state.projects.find((snapshot) => snapshot.tasks.some((task) => task.id === taskId));
}

function findSection(state: ClientState, id: string): Section | undefined {
  for (const snapshot of state.projects) {
    const section = snapshot.sections.find((existing) => existing.id === id);
    if (section) {
      return section;
    }
  }
  return undefined;
}

function findTask(state: ClientState, id: string): Task | undefined {
  for (const snapshot of state.projects) {
    const task = snapshot.tasks.find((existing) => existing.id === id);
    if (task) {
      return task;
    }
  }
  return undefined;
}

function findComment(state: ClientState, id: string): Comment | undefined {
  for (const snapshot of state.projects) {
    const comment = snapshot.comments.find((existing) => existing.id === id);
    if (comment) {
      return comment;
    }
  }
  return undefined;
}

function findAutomation(state: ClientState, id: string): Automation | undefined {
  for (const snapshot of state.projects) {
    const automation = snapshot.automations.find((existing) => existing.id === id);
    if (automation) {
      return automation;
    }
  }
  return undefined;
}

function removeSection(state: ClientState, id: string): void {
  for (const snapshot of state.projects) {
    const index = snapshot.sections.findIndex((section) => section.id === id);
    if (index !== -1) {
      snapshot.sections.splice(index, 1);
      for (const task of snapshot.tasks) {
        if (task.section_id === id) {
          task.section_id = null;
        }
      }
      return;
    }
  }
}

function removeTask(state: ClientState, id: string): void {
  for (const snapshot of state.projects) {
    const index = snapshot.tasks.findIndex((task) => task.id === id);
    if (index !== -1) {
      snapshot.tasks.splice(index, 1);
      snapshot.comments = snapshot.comments.filter((comment) => comment.task_id !== id);
      return;
    }
  }
}

function removeComment(state: ClientState, id: string): void {
  for (const snapshot of state.projects) {
    snapshot.comments = snapshot.comments.filter((comment) => comment.id !== id);
  }
}

function removeAutomation(state: ClientState, id: string): void {
  for (const snapshot of state.projects) {
    snapshot.automations = snapshot.automations.filter((automation) => automation.id !== id);
  }
}
