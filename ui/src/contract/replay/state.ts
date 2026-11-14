import type { Automation } from '../../generated/Automation';
import type { Comment } from '../../generated/Comment';
import type { Project } from '../../generated/Project';
import type { ProjectSnapshot } from '../../generated/ProjectSnapshot';
import type { Section } from '../../generated/Section';
import type { Task } from '../../generated/Task';
import type { ClientState } from './types';

export function findProject(state: ClientState, id: string): ProjectSnapshot | undefined {
  return state.projects.find((snapshot) => snapshot.id === id);
}

export function findProjectForTask(
  state: ClientState,
  taskId: string,
): ProjectSnapshot | undefined {
  return state.projects.find((snapshot) => snapshot.tasks.some((task) => task.id === taskId));
}

export function findSection(state: ClientState, id: string): Section | undefined {
  for (const snapshot of state.projects) {
    const section = snapshot.sections.find((existing) => existing.id === id);
    if (section) {
      return section;
    }
  }
  return undefined;
}

export function findTask(state: ClientState, id: string): Task | undefined {
  for (const snapshot of state.projects) {
    const task = snapshot.tasks.find((existing) => existing.id === id);
    if (task) {
      return task;
    }
  }
  return undefined;
}

export function findComment(state: ClientState, id: string): Comment | undefined {
  for (const snapshot of state.projects) {
    const comment = snapshot.comments.find((existing) => existing.id === id);
    if (comment) {
      return comment;
    }
  }
  return undefined;
}

export function findAutomation(state: ClientState, id: string): Automation | undefined {
  for (const snapshot of state.projects) {
    const automation = snapshot.automations.find((existing) => existing.id === id);
    if (automation) {
      return automation;
    }
  }
  return undefined;
}

export function setProject(snapshot: ProjectSnapshot, project: Project): void {
  snapshot.id = project.id;
  snapshot.name = project.name;
  snapshot.description = project.description;
  snapshot.created_at = project.created_at;
  snapshot.updated_at = project.updated_at;
}

export function upsertSection(snapshot: ProjectSnapshot, section: Section): void {
  const index = snapshot.sections.findIndex((existing) => existing.id === section.id);
  if (index === -1) {
    snapshot.sections.push(section);
  } else {
    snapshot.sections[index] = section;
  }
}

export function upsertTask(snapshot: ProjectSnapshot, task: Task): void {
  const index = snapshot.tasks.findIndex((existing) => existing.id === task.id);
  if (index === -1) {
    snapshot.tasks.push(task);
  } else {
    snapshot.tasks[index] = task;
  }
}

export function upsertComment(snapshot: ProjectSnapshot, comment: Comment): void {
  const index = snapshot.comments.findIndex((existing) => existing.id === comment.id);
  if (index === -1) {
    snapshot.comments.push(comment);
  } else {
    snapshot.comments[index] = comment;
  }
}

export function upsertAutomation(snapshot: ProjectSnapshot, automation: Automation): void {
  const index = snapshot.automations.findIndex((existing) => existing.id === automation.id);
  if (index === -1) {
    snapshot.automations.push(automation);
  } else {
    snapshot.automations[index] = automation;
  }
}

export function removeSection(state: ClientState, id: string): void {
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

export function removeTask(state: ClientState, id: string): void {
  for (const snapshot of state.projects) {
    const index = snapshot.tasks.findIndex((task) => task.id === id);
    if (index !== -1) {
      snapshot.tasks.splice(index, 1);
      snapshot.comments = snapshot.comments.filter((comment) => comment.task_id !== id);
      return;
    }
  }
}

export function removeComment(state: ClientState, id: string): void {
  for (const snapshot of state.projects) {
    snapshot.comments = snapshot.comments.filter((comment) => comment.id !== id);
  }
}

export function removeAutomation(state: ClientState, id: string): void {
  for (const snapshot of state.projects) {
    snapshot.automations = snapshot.automations.filter((automation) => automation.id !== id);
  }
}
