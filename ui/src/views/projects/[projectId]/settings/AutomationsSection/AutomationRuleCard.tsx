import { Button } from 'flowbite-react';
import { HiTrash } from 'react-icons/hi2';
import type { Automation } from '@/generated/Automation';
import type { AutomationAction } from '@/generated/AutomationAction';
import type { Section } from '@/generated/Section';
import { automationTriggerLabel } from '@/utils/automations/operations';
import { ActionEditor } from './ActionEditor';

export type AutomationRuleCardProps = {
  automation: Automation;
  sections: readonly Section[];
  onChange: (action: AutomationAction) => void;
  onDelete: () => void;
};

export function AutomationRuleCard(props: AutomationRuleCardProps) {
  const { automation, sections, onChange, onDelete } = props;

  const label = automationTriggerLabel(automation.trigger, sections);
  const action = automation.actions[0];

  return (
    <li className="flex flex-col gap-3 px-3 py-3 md:flex-row md:items-center md:gap-4">
      <span className="min-w-0 font-medium text-white md:w-56 md:shrink-0">{label}</span>
      <div className="md:min-w-0 md:flex-1">
        <ActionEditor action={action} sections={sections} label={label} onChange={onChange} />
      </div>
      <Button
        color="red"
        size="sm"
        aria-label={`Remove ${label}`}
        onClick={onDelete}
        className="min-h-11 min-w-11 self-start md:min-h-0 md:min-w-0 md:self-auto"
      >
        <HiTrash className="h-5 w-5" />
      </Button>
    </li>
  );
}
