import type { Automation } from '@/generated/Automation';
import type { AutomationAction } from '@/generated/AutomationAction';
import type { AutomationTrigger } from '@/generated/AutomationTrigger';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { useReplica } from '@/providers/Replica';
import {
  automationTriggerKey,
  buildAutomationOperation,
  deleteAutomationOperation,
  findAutomation,
} from '@/utils/automations/operations';
import { AddAutomationForm } from './AddAutomationForm';
import { AutomationRuleCard } from './AutomationRuleCard';

const FIXED_TRIGGERS: readonly AutomationTrigger[] = [
  { kind: 'task_created' },
  { kind: 'task_closed' },
  { kind: 'task_opened' },
];

export type AutomationsSectionProps = {
  project: ProjectSnapshot;
};

export function AutomationsSection(props: AutomationsSectionProps) {
  const { project } = props;

  const { enqueue } = useReplica();

  const sections = [...project.sections].sort((left, right) => {
    if (left.position < right.position) {
      return -1;
    }
    if (left.position > right.position) {
      return 1;
    }
    return 0;
  });

  const orderedTriggers: AutomationTrigger[] = [
    ...FIXED_TRIGGERS,
    ...sections.map((section) => ({
      kind: 'task_enters_section' as const,
      section_id: section.id,
    })),
  ];

  const rules = orderedTriggers
    .map((trigger) => findAutomation(project, trigger))
    .filter((automation): automation is Automation => automation !== undefined);

  const knownTriggerKeys = new Set(orderedTriggers.map(automationTriggerKey));
  const extraRules = project.automations.filter(
    (automation) => !knownTriggerKeys.has(automationTriggerKey(automation.trigger)),
  );
  const orderedRules = [...rules, ...extraRules];

  const availableTriggers = orderedTriggers.filter(
    (trigger) => findAutomation(project, trigger) === undefined,
  );

  function applyOperation(operation: Operation | null) {
    if (operation !== null) {
      void enqueue(operation);
    }
  }

  function changeRuleAction(automation: Automation, action: AutomationAction) {
    applyOperation(buildAutomationOperation(project, automation.trigger, action));
  }

  function handleAdd(trigger: AutomationTrigger, action: AutomationAction) {
    applyOperation(buildAutomationOperation(project, trigger, action));
  }

  return (
    <div className="flex flex-col gap-2">
      <p className="text-muted text-sm">
        One rule per trigger. Actions do not trigger other automations.
      </p>
      {orderedRules.length === 0 ? (
        <p className="text-muted text-sm">No automations yet.</p>
      ) : (
        <ul className="divide-y divide-zinc-700 rounded-lg border border-zinc-700">
          {orderedRules.map((automation) => (
            <AutomationRuleCard
              key={automation.id}
              automation={automation}
              sections={sections}
              onChange={(action) => changeRuleAction(automation, action)}
              onDelete={() => applyOperation(deleteAutomationOperation(automation))}
            />
          ))}
        </ul>
      )}
      {availableTriggers.length === 0 ? (
        <p className="text-muted text-sm">Every trigger already has a rule.</p>
      ) : (
        <AddAutomationForm
          project={project}
          sections={sections}
          orderedTriggers={orderedTriggers}
          availableTriggers={availableTriggers}
          onAdd={handleAdd}
        />
      )}
    </div>
  );
}
