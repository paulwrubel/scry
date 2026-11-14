import type { Automation } from '../../generated/Automation';
import type { Comment } from '../../generated/Comment';
import type { Operation } from '../../generated/Operation';
import type { Project } from '../../generated/Project';
import type { Section } from '../../generated/Section';
import type { Task } from '../../generated/Task';
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

export function applyOperation(state: ClientState, operation: Operation): void {
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
