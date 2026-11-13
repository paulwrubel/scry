import type { Automation } from '@/generated/Automation';
import type { AutomationAction } from '@/generated/AutomationAction';
import type { AutomationTrigger } from '@/generated/AutomationTrigger';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import { newId } from '@/utils/uuid';

/** Mirrors `MAX_AUTOMATION_ACTIONS` in the Rust core. */
export const MAX_AUTOMATION_ACTIONS = 1;

const PRIORITY_LABELS = {
  critical: 'Critical',
  elevated: 'Elevated',
  normal: 'Normal',
} as const;

export function sameAutomationTrigger(a: AutomationTrigger, b: AutomationTrigger): boolean {
  if (a.kind !== b.kind) {
    return false;
  }
  if (a.kind === 'task_enters_section' && b.kind === 'task_enters_section') {
    return a.section_id === b.section_id;
  }
  return true;
}

export function automationTriggerKey(trigger: AutomationTrigger): string {
  return trigger.kind === 'task_enters_section'
    ? `task_enters_section:${trigger.section_id}`
    : trigger.kind;
}

export function sameAutomationAction(a: AutomationAction, b: AutomationAction): boolean {
  if (a.kind !== b.kind) {
    return false;
  }
  if (a.kind === 'move_to_section' && b.kind === 'move_to_section') {
    return a.section_id === b.section_id;
  }
  if (a.kind === 'set_priority' && b.kind === 'set_priority') {
    return a.priority === b.priority;
  }
  return true;
}

export function findAutomation(
  project: ProjectSnapshot,
  trigger: AutomationTrigger,
): Automation | undefined {
  return project.automations.find((automation) =>
    sameAutomationTrigger(automation.trigger, trigger),
  );
}

export function automationReferencesSection(automation: Automation, sectionId: string): boolean {
  if (
    automation.trigger.kind === 'task_enters_section' &&
    automation.trigger.section_id === sectionId
  ) {
    return true;
  }
  return automation.actions.some(
    (action) => action.kind === 'move_to_section' && action.section_id === sectionId,
  );
}

export function automationTriggerLabel(
  trigger: AutomationTrigger,
  sections: readonly Section[],
): string {
  switch (trigger.kind) {
    case 'task_created':
      return 'When a task is created';
    case 'task_closed':
      return 'When a task is closed';
    case 'task_opened':
      return 'When a task is opened';
    case 'task_enters_section': {
      const section = sections.find((candidate) => candidate.id === trigger.section_id);
      return `When a task enters ${section?.name ?? 'a section'}`;
    }
  }
}

export function automationActionLabel(
  action: AutomationAction,
  sections: readonly Section[],
): string {
  switch (action.kind) {
    case 'move_to_section': {
      if (action.section_id === null) {
        return 'move to No Section';
      }
      const section = sections.find((candidate) => candidate.id === action.section_id);
      return `move to ${section?.name ?? 'a section'}`;
    }
    case 'close_task':
      return 'close it';
    case 'open_task':
      return 'open it';
    case 'set_priority':
      return `set priority to ${PRIORITY_LABELS[action.priority]}`;
  }
}

export function describeAutomation(automation: Automation, sections: readonly Section[]): string {
  const trigger = automationTriggerLabel(automation.trigger, sections);
  const actions = automation.actions.map((action) => automationActionLabel(action, sections));
  return `${trigger} → ${actions.join(', ')}`;
}

export function createAutomationOperation(
  projectId: string,
  trigger: AutomationTrigger,
  actions: AutomationAction[],
): Operation {
  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'create_automation',
    id: newId(),
    project_id: projectId,
    trigger,
    actions,
    created_at: new Date().toISOString(),
  };
}

export function updateAutomationOperation(
  automation: Automation,
  actions: AutomationAction[],
): Operation {
  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'update_automation',
    id: automation.id,
    actions,
  };
}

export function deleteAutomationOperation(automation: Automation): Operation {
  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'delete_automation',
    id: automation.id,
  };
}

export function buildAutomationOperation(
  project: ProjectSnapshot,
  trigger: AutomationTrigger,
  action: AutomationAction | null,
): Operation | null {
  const existing = findAutomation(project, trigger);

  if (action === null) {
    return existing === undefined ? null : deleteAutomationOperation(existing);
  }

  if (existing !== undefined) {
    const current = existing.actions[0];
    if (current !== undefined && sameAutomationAction(current, action)) {
      return null;
    }
    return updateAutomationOperation(existing, [action]);
  }

  return createAutomationOperation(project.id, trigger, [action]);
}
