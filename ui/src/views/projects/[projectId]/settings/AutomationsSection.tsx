import {
  Button,
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeadCell,
  TableRow,
} from 'flowbite-react';
import { useState } from 'react';
import { HiPlus, HiTrash } from 'react-icons/hi2';
import type { Automation } from '@/generated/Automation';
import type { AutomationAction } from '@/generated/AutomationAction';
import type { AutomationTrigger } from '@/generated/AutomationTrigger';
import type { Operation } from '@/generated/Operation';
import type { Priority } from '@/generated/Priority';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import { useReplica } from '@/providers/Replica';
import {
  automationTriggerKey,
  automationTriggerLabel,
  buildAutomationOperation,
  deleteAutomationOperation,
  findAutomation,
} from '@/utils/automationOperations';

const FIXED_TRIGGERS: readonly AutomationTrigger[] = [
  { kind: 'task_created' },
  { kind: 'task_closed' },
  { kind: 'task_opened' },
];

const ACTION_OPTIONS = [
  { label: 'Move to', value: 'move_to_section' },
  { label: 'Close', value: 'close_task' },
  { label: 'Open', value: 'open_task' },
  { label: 'Set priority', value: 'set_priority' },
] as const;

const PRIORITY_OPTIONS: readonly { label: string; value: Priority }[] = [
  { label: 'Critical', value: 'critical' },
  { label: 'High', value: 'high' },
  { label: 'Medium', value: 'medium' },
  { label: 'Low', value: 'low' },
  { label: 'Minimal', value: 'minimal' },
];

const tableTheme = {
  root: { base: 'w-full text-left text-sm text-zinc-300' },
  head: {
    base: 'text-xs uppercase text-zinc-400',
    cell: {
      base: 'border-b border-zinc-700 bg-transparent px-3 py-2 font-medium',
    },
  },
  body: { cell: { base: 'px-3 py-2' } },
};

export type AutomationsSectionProps = {
  project: ProjectSnapshot;
};

export function AutomationsSection(props: AutomationsSectionProps) {
  const { project } = props;
  const replica = useReplica();

  const sections = [...project.sections].sort((left, right) => {
    if (left.position < right.position) {
      return -1;
    }
    if (left.position > right.position) {
      return 1;
    }
    return 0;
  });

  const [selectedTriggerKey, setSelectedTriggerKey] = useState('');
  const [selectedAction, setSelectedAction] = useState<AutomationAction>(() =>
    defaultAutomationAction(sections),
  );

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
  const selectedTrigger =
    availableTriggers.find((trigger) => automationTriggerKey(trigger) === selectedTriggerKey) ??
    availableTriggers.at(0);

  function enqueue(operation: Operation | null) {
    if (operation !== null) {
      void replica.enqueue(operation);
    }
  }

  function changeRuleAction(automation: Automation, action: AutomationAction) {
    enqueue(buildAutomationOperation(project, automation.trigger, action));
  }

  function handleAdd() {
    if (selectedTrigger === undefined) {
      return;
    }
    enqueue(buildAutomationOperation(project, selectedTrigger, selectedAction));
    setSelectedTriggerKey('');
    setSelectedAction(defaultAutomationAction(sections));
  }

  return (
    <div className="flex flex-col gap-2">
      <p className="text-muted text-sm">
        One rule per trigger. Actions do not trigger other automations.
      </p>
      {orderedRules.length === 0 ? (
        <p className="text-muted text-sm">No automations yet.</p>
      ) : (
        <div className="overflow-x-auto rounded-lg border border-zinc-700">
          <Table theme={tableTheme}>
            <TableHead>
              <TableRow>
                <TableHeadCell className="w-56">Trigger</TableHeadCell>
                <TableHeadCell>Action</TableHeadCell>
                <TableHeadCell className="w-12">
                  <span className="sr-only">Remove</span>
                </TableHeadCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {orderedRules.map((automation) => {
                const label = automationTriggerLabel(automation.trigger, sections);
                const action = automation.actions[0];

                return (
                  <TableRow key={automation.id}>
                    <TableCell className="font-medium whitespace-nowrap text-white">
                      {label}
                    </TableCell>
                    <TableCell>
                      <ActionEditor
                        action={action}
                        sections={sections}
                        label={label}
                        onChange={(next) => changeRuleAction(automation, next)}
                      />
                    </TableCell>
                    <TableCell className="w-12">
                      <Button
                        color="red"
                        size="sm"
                        aria-label={`Remove ${label}`}
                        onClick={() => enqueue(deleteAutomationOperation(automation))}
                      >
                        <HiTrash className="h-5 w-5" />
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </div>
      )}
      {availableTriggers.length === 0 ? (
        <p className="text-muted text-sm">Every trigger already has a rule.</p>
      ) : (
        <div className="flex flex-col gap-3 rounded-lg border border-zinc-700 p-3">
          <h3 className="text-sm font-medium text-zinc-300">Add automation</h3>
          <div className="flex flex-col gap-2 md:flex-row md:items-center">
            <Select
              sizing="sm"
              aria-label="New automation trigger"
              className="md:w-64"
              value={selectedTrigger === undefined ? '' : automationTriggerKey(selectedTrigger)}
              onChange={(event) => setSelectedTriggerKey(event.target.value)}
            >
              {orderedTriggers.map((trigger) => {
                const key = automationTriggerKey(trigger);
                return (
                  <option
                    key={key}
                    value={key}
                    disabled={findAutomation(project, trigger) !== undefined}
                  >
                    {automationTriggerLabel(trigger, sections)}
                  </option>
                );
              })}
            </Select>
            <ActionEditor
              action={selectedAction}
              sections={sections}
              label="the new automation"
              onChange={setSelectedAction}
            />
            <Button color="light" size="sm" onClick={handleAdd}>
              <HiPlus className="mr-1 h-4 w-4" />
              Add
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

type ActionEditorProps = {
  action: AutomationAction;
  sections: readonly Section[];
  label: string;
  onChange: (action: AutomationAction) => void;
};

function ActionEditor(props: ActionEditorProps) {
  const { action, sections, label, onChange } = props;
  const kind = action.kind;
  const moveSectionId =
    action.kind === 'move_to_section' ? action.section_id : (sections[0]?.id ?? null);
  const priority = action.kind === 'set_priority' ? action.priority : 'medium';

  function changeKind(value: string) {
    switch (value) {
      case 'move_to_section': {
        const sectionId =
          action.kind === 'move_to_section' ? action.section_id : (sections[0]?.id ?? null);
        onChange({ kind: 'move_to_section', section_id: sectionId });
        return;
      }
      case 'close_task':
        onChange({ kind: 'close_task' });
        return;
      case 'open_task':
        onChange({ kind: 'open_task' });
        return;
      case 'set_priority': {
        const nextPriority = action.kind === 'set_priority' ? action.priority : 'medium';
        onChange({ kind: 'set_priority', priority: nextPriority });
        return;
      }
      default:
        return;
    }
  }

  return (
    <div className="flex flex-col gap-2 md:flex-row md:items-center">
      <Select
        sizing="sm"
        className="md:w-40"
        aria-label={`Action for ${label}`}
        value={kind}
        onChange={(event) => changeKind(event.target.value)}
      >
        {ACTION_OPTIONS.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </Select>
      {kind === 'move_to_section' ? (
        <Select
          sizing="sm"
          className="md:w-36"
          aria-label={`Section for ${label}`}
          value={moveSectionId ?? ''}
          onChange={(event) =>
            onChange({
              kind: 'move_to_section',
              section_id: event.target.value === '' ? null : event.target.value,
            })
          }
        >
          <option value="">No section</option>
          {sections.map((section) => (
            <option key={section.id} value={section.id}>
              {section.name}
            </option>
          ))}
        </Select>
      ) : null}
      {kind === 'set_priority' ? (
        <Select
          sizing="sm"
          className="md:w-36"
          aria-label={`Priority for ${label}`}
          value={priority}
          onChange={(event) =>
            onChange({ kind: 'set_priority', priority: event.target.value as Priority })
          }
        >
          {PRIORITY_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </Select>
      ) : null}
    </div>
  );
}

function defaultAutomationAction(sections: readonly Section[]): AutomationAction {
  return { kind: 'move_to_section', section_id: sections[0]?.id ?? null };
}
