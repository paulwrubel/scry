import { describe, expect, it } from 'vitest';
import type { Automation } from '@/generated/Automation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import {
  automationActionLabel,
  automationReferencesSection,
  automationTriggerKey,
  automationTriggerLabel,
  buildAutomationOperation,
  createAutomationOperation,
  deleteAutomationOperation,
  describeAutomation,
  findAutomation,
  sameAutomationAction,
  sameAutomationTrigger,
  updateAutomationOperation,
} from './automationOperations';

const PROJECT_ID = 'project-1';
const SECTION_A_ID = 'section-a';
const SECTION_B_ID = 'section-b';
const AUTOMATION_ID = 'automation-1';
const CREATED_AT = '2026-01-01T00:00:00Z';

const SECTION_A: Section = {
  id: SECTION_A_ID,
  project_id: PROJECT_ID,
  name: 'Todo',
  position: 'a0',
  color: null,
  created_at: CREATED_AT,
};

const SECTION_B: Section = {
  id: SECTION_B_ID,
  project_id: PROJECT_ID,
  name: 'Doing',
  position: 'a1',
  color: null,
  created_at: CREATED_AT,
};

function automation(overrides: Partial<Automation> = {}): Automation {
  return {
    id: AUTOMATION_ID,
    project_id: PROJECT_ID,
    trigger: { kind: 'task_created' },
    actions: [{ kind: 'close_task' }],
    created_at: CREATED_AT,
    ...overrides,
  };
}

function project(automations: Automation[] = []): ProjectSnapshot {
  return {
    id: PROJECT_ID,
    name: 'Project',
    description: null,
    created_at: CREATED_AT,
    sections: [SECTION_A, SECTION_B],
    tasks: [],
    notes: [],
    automations,
  };
}

describe('automation equality', () => {
  it('compares trigger kinds and section ids', () => {
    expect(sameAutomationTrigger({ kind: 'task_created' }, { kind: 'task_created' })).toBe(true);
    expect(sameAutomationTrigger({ kind: 'task_created' }, { kind: 'task_closed' })).toBe(false);
    expect(
      sameAutomationTrigger(
        { kind: 'task_enters_section', section_id: SECTION_A_ID },
        { kind: 'task_enters_section', section_id: SECTION_A_ID },
      ),
    ).toBe(true);
    expect(
      sameAutomationTrigger(
        { kind: 'task_enters_section', section_id: SECTION_A_ID },
        { kind: 'task_enters_section', section_id: SECTION_B_ID },
      ),
    ).toBe(false);
  });

  it('compares action kinds and their parameters', () => {
    expect(sameAutomationAction({ kind: 'close_task' }, { kind: 'close_task' })).toBe(true);
    expect(sameAutomationAction({ kind: 'close_task' }, { kind: 'open_task' })).toBe(false);
    expect(
      sameAutomationAction(
        { kind: 'move_to_section', section_id: null },
        { kind: 'move_to_section', section_id: null },
      ),
    ).toBe(true);
    expect(
      sameAutomationAction(
        { kind: 'move_to_section', section_id: SECTION_A_ID },
        { kind: 'move_to_section', section_id: SECTION_B_ID },
      ),
    ).toBe(false);
    expect(
      sameAutomationAction(
        { kind: 'set_priority', priority: 'high' },
        { kind: 'set_priority', priority: 'high' },
      ),
    ).toBe(true);
    expect(
      sameAutomationAction(
        { kind: 'set_priority', priority: 'high' },
        { kind: 'set_priority', priority: 'low' },
      ),
    ).toBe(false);
  });
});

describe('automationTriggerKey', () => {
  it('keys section triggers by section and others by kind', () => {
    expect(automationTriggerKey({ kind: 'task_created' })).toBe('task_created');
    expect(automationTriggerKey({ kind: 'task_closed' })).toBe('task_closed');
    expect(automationTriggerKey({ kind: 'task_opened' })).toBe('task_opened');

    const sectionKey = automationTriggerKey({
      kind: 'task_enters_section',
      section_id: SECTION_A_ID,
    });
    expect(sectionKey).toBe(`task_enters_section:${SECTION_A_ID}`);
  });
});

describe('automationReferencesSection', () => {
  it('detects trigger and action references', () => {
    const triggered = automation({
      trigger: { kind: 'task_enters_section', section_id: SECTION_A_ID },
    });
    expect(automationReferencesSection(triggered, SECTION_A_ID)).toBe(true);
    expect(automationReferencesSection(triggered, SECTION_B_ID)).toBe(false);

    const moved = automation({
      actions: [{ kind: 'move_to_section', section_id: SECTION_A_ID }],
    });
    expect(automationReferencesSection(moved, SECTION_A_ID)).toBe(true);
    expect(automationReferencesSection(automation(), SECTION_A_ID)).toBe(false);
  });
});

describe('automation labels', () => {
  const sections = [SECTION_A, SECTION_B];

  it('labels triggers', () => {
    const created = automationTriggerLabel({ kind: 'task_created' }, sections);
    const closed = automationTriggerLabel({ kind: 'task_closed' }, sections);
    const opened = automationTriggerLabel({ kind: 'task_opened' }, sections);
    const enters = automationTriggerLabel(
      { kind: 'task_enters_section', section_id: SECTION_A_ID },
      sections,
    );

    expect(created).toBe('When a task is created');
    expect(closed).toBe('When a task is closed');
    expect(opened).toBe('When a task is opened');
    expect(enters).toBe('When a task enters Todo');
  });

  it('labels actions', () => {
    const move = automationActionLabel(
      { kind: 'move_to_section', section_id: SECTION_A_ID },
      sections,
    );
    const noSection = automationActionLabel(
      { kind: 'move_to_section', section_id: null },
      sections,
    );
    const close = automationActionLabel({ kind: 'close_task' }, sections);
    const open = automationActionLabel({ kind: 'open_task' }, sections);
    const priority = automationActionLabel({ kind: 'set_priority', priority: 'high' }, sections);

    expect(move).toBe('move to Todo');
    expect(noSection).toBe('move to No Section');
    expect(close).toBe('close it');
    expect(open).toBe('open it');
    expect(priority).toBe('set priority to High');
  });

  it('describes a rule', () => {
    const description = describeAutomation(
      automation({ actions: [{ kind: 'move_to_section', section_id: SECTION_B_ID }] }),
      sections,
    );

    expect(description).toBe('When a task is created → move to Doing');
  });
});

describe('automation operations', () => {
  it('builds create, update, and delete operations', () => {
    const create = createAutomationOperation(PROJECT_ID, { kind: 'task_created' }, [
      { kind: 'close_task' },
    ]);
    expect(create).toMatchObject({
      operation_version: 1,
      kind: 'create_automation',
      project_id: PROJECT_ID,
      trigger: { kind: 'task_created' },
      actions: [{ kind: 'close_task' }],
      created_at: expect.any(String),
    });
    expect(create.id).not.toBe(create.operation_id);

    const existing = automation();
    const update = updateAutomationOperation(existing, [{ kind: 'open_task' }]);
    expect(update).toMatchObject({
      operation_version: 1,
      kind: 'update_automation',
      id: AUTOMATION_ID,
      actions: [{ kind: 'open_task' }],
    });

    const remove = deleteAutomationOperation(existing);
    expect(remove).toMatchObject({
      operation_version: 1,
      kind: 'delete_automation',
      id: AUTOMATION_ID,
    });
  });

  it('builds the operation needed to change one trigger rule', () => {
    const empty = project();
    const create = buildAutomationOperation(
      empty,
      { kind: 'task_created' },
      { kind: 'close_task' },
    );
    expect(create).toMatchObject({ kind: 'create_automation' });

    const withRule = project([automation()]);
    expect(findAutomation(withRule, { kind: 'task_created' })).toEqual(automation());

    const unchanged = buildAutomationOperation(
      withRule,
      { kind: 'task_created' },
      { kind: 'close_task' },
    );
    expect(unchanged).toBeNull();

    const update = buildAutomationOperation(
      withRule,
      { kind: 'task_created' },
      { kind: 'open_task' },
    );
    expect(update).toMatchObject({ kind: 'update_automation' });

    const remove = buildAutomationOperation(withRule, { kind: 'task_created' }, null);
    expect(remove).toMatchObject({ kind: 'delete_automation' });

    const none = buildAutomationOperation(empty, { kind: 'task_created' }, null);
    expect(none).toBeNull();
  });
});
