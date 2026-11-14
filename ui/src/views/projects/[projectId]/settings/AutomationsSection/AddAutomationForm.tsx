import { Button, Select } from 'flowbite-react';
import { HiPlus } from 'react-icons/hi2';
import type { AutomationAction } from '@/generated/AutomationAction';
import type { AutomationTrigger } from '@/generated/AutomationTrigger';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import { useAppForm } from '@/hooks/useAppForm';
import {
  automationTriggerKey,
  automationTriggerLabel,
  findAutomation,
} from '@/utils/automations/operations';
import { ActionEditor } from './ActionEditor';

function defaultAutomationAction(sections: readonly Section[]): AutomationAction {
  return { kind: 'move_to_section', section_id: sections[0]?.id ?? null };
}

export type AddAutomationFormProps = {
  project: ProjectSnapshot;
  sections: readonly Section[];
  orderedTriggers: readonly AutomationTrigger[];
  availableTriggers: readonly AutomationTrigger[];
  onAdd: (trigger: AutomationTrigger, action: AutomationAction) => void;
};

export function AddAutomationForm(props: AddAutomationFormProps) {
  const { project, sections, orderedTriggers, availableTriggers, onAdd } = props;

  const form = useAppForm({
    defaultValues: { triggerKey: '', action: defaultAutomationAction(sections) },
    onSubmit: ({ value }) => {
      const trigger =
        availableTriggers.find(
          (candidate) => automationTriggerKey(candidate) === value.triggerKey,
        ) ?? availableTriggers.at(0);
      if (trigger === undefined) {
        return;
      }
      onAdd(trigger, value.action);
      form.setFieldValue('triggerKey', '');
      form.setFieldValue('action', defaultAutomationAction(sections));
    },
  });

  const firstAvailable = availableTriggers.at(0);
  const firstAvailableKey =
    firstAvailable === undefined ? '' : automationTriggerKey(firstAvailable);

  return (
    <div className="flex flex-col gap-3 rounded-lg border border-zinc-700 p-3">
      <h3 className="text-sm font-medium text-zinc-300">Add automation</h3>
      <form
        className="flex flex-col gap-2 md:flex-row md:items-center"
        onSubmit={(event) => {
          event.preventDefault();
          void form.handleSubmit();
        }}
      >
        <form.AppField name="triggerKey">
          {(field) => (
            <Select
              sizing="sm"
              aria-label="New automation trigger"
              className="md:w-64 [&_select]:min-h-11 md:[&_select]:min-h-0"
              value={field.state.value === '' ? firstAvailableKey : field.state.value}
              onChange={(event) => field.handleChange(event.target.value)}
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
          )}
        </form.AppField>
        <form.AppField name="action">
          {(field) => (
            <ActionEditor
              action={field.state.value}
              sections={sections}
              label="the new automation"
              onChange={(next) => field.handleChange(next)}
            />
          )}
        </form.AppField>
        <Button
          color="light"
          size="sm"
          type="submit"
          className="min-h-11 w-full md:min-h-0 md:w-auto"
        >
          <HiPlus className="mr-1 h-4 w-4" />
          Add
        </Button>
      </form>
    </div>
  );
}
