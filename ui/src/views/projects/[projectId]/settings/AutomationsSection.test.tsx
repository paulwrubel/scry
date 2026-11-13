// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Automation } from '@/generated/Automation';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import type { Replica } from '@/utils/replica';
import { AutomationsSection } from './AutomationsSection';

const PROJECT_ID = 'project-1';
const SECTION_ID = 'section-1';
const AUTOMATION_ID = 'automation-1';
const CREATED_AT = '2026-01-01T00:00:00Z';

function projectSnapshot(overrides: Partial<ProjectSnapshot> = {}): ProjectSnapshot {
  return {
    id: PROJECT_ID,
    name: 'Project',
    description: null,
    show_priority: false,
    created_at: CREATED_AT,
    sections: [
      {
        id: SECTION_ID,
        project_id: PROJECT_ID,
        name: 'Todo',
        position: 'a0',
        color: null,
        created_at: CREATED_AT,
      },
    ],
    tasks: [],
    notes: [],
    automations: [],
    ...overrides,
  };
}

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

function renderSection(project: ProjectSnapshot, enqueue: (operation: Operation) => Promise<void>) {
  const snapshot = {
    confirmed: { projects: [project] },
    visible: { projects: [project] },
    outbox: [],
    needsBootstrap: false,
    persisted: true,
    lastSeenSeq: 0,
  };

  const context: ReplicaContextValue = {
    status: 'ready',
    replica: {
      getSnapshot: () => snapshot,
      subscribe: () => () => undefined,
      enqueue,
    } as unknown as Replica,
    error: undefined,
    retry: () => undefined,
  };

  render(
    <ReplicaContext.Provider value={context}>
      <AutomationsSection project={project} />
    </ReplicaContext.Provider>,
  );
}

function addButton() {
  return screen.getByRole('button', { name: 'Add' }) as HTMLButtonElement;
}

afterEach(cleanup);

describe('AutomationsSection', () => {
  it('has no placeholder choices in the add form', () => {
    renderSection(
      projectSnapshot(),
      vi.fn(async () => undefined),
    );

    expect(screen.queryByText('Choose a trigger')).toBeNull();
    expect(screen.queryByText('Choose an action')).toBeNull();
  });

  it('defaults the add form to the first available trigger and action', () => {
    renderSection(
      projectSnapshot(),
      vi.fn(async () => undefined),
    );

    const trigger = screen.getByLabelText('New automation trigger') as HTMLSelectElement;
    const action = screen.getByLabelText('Action for the new automation') as HTMLSelectElement;

    expect(trigger.value).toBe('task_created');
    expect(action.value).toBe('move_to_section');
    expect(addButton().disabled).toBe(false);
  });

  it('disables triggers that already have a rule', () => {
    renderSection(
      projectSnapshot({ automations: [automation()] }),
      vi.fn(async () => undefined),
    );

    const option = screen.getByRole('option', {
      name: 'When a task is created',
    }) as HTMLOptionElement;

    expect(option.disabled).toBe(true);
  });

  it('enqueues a create with the effective trigger and action', async () => {
    const enqueue = vi.fn(async (_operation: Operation) => undefined);
    renderSection(projectSnapshot(), enqueue);

    fireEvent.change(screen.getByLabelText('New automation trigger'), {
      target: { value: 'task_closed' },
    });
    fireEvent.click(addButton());

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledTimes(1);
    });
    expect(enqueue).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'create_automation',
        project_id: PROJECT_ID,
        trigger: { kind: 'task_closed' },
        actions: [{ kind: 'move_to_section', section_id: SECTION_ID }],
      }),
    );
  });

  it('enqueues an update when an existing rule action changes', async () => {
    const enqueue = vi.fn(async (_operation: Operation) => undefined);
    renderSection(projectSnapshot({ automations: [automation()] }), enqueue);

    fireEvent.change(screen.getByLabelText('Action for When a task is created'), {
      target: { value: 'open_task' },
    });

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledTimes(1);
    });
    expect(enqueue).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'update_automation',
        id: AUTOMATION_ID,
        actions: [{ kind: 'open_task' }],
      }),
    );
  });

  it('enqueues a delete when a rule is removed', async () => {
    const enqueue = vi.fn(async (_operation: Operation) => undefined);
    renderSection(projectSnapshot({ automations: [automation()] }), enqueue);

    fireEvent.click(screen.getByRole('button', { name: 'Remove When a task is created' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledTimes(1);
    });
    expect(enqueue).toHaveBeenCalledWith(
      expect.objectContaining({ kind: 'delete_automation', id: AUTOMATION_ID }),
    );
  });

  it('hides the add form when every trigger has a rule', () => {
    const automations = [
      automation({ id: 'automation-1', trigger: { kind: 'task_created' } }),
      automation({ id: 'automation-2', trigger: { kind: 'task_closed' } }),
      automation({ id: 'automation-3', trigger: { kind: 'task_opened' } }),
      automation({
        id: 'automation-4',
        trigger: { kind: 'task_enters_section', section_id: SECTION_ID },
      }),
    ];
    renderSection(
      projectSnapshot({ automations }),
      vi.fn(async () => undefined),
    );

    expect(screen.getByText('Every trigger already has a rule.')).toBeDefined();
    expect(screen.queryByLabelText('New automation trigger')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Add' })).toBeNull();
  });
});
