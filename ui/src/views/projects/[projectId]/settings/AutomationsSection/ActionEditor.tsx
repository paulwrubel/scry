import { Select } from 'flowbite-react';
import type { AutomationAction } from '@/generated/AutomationAction';
import type { Priority } from '@/generated/Priority';
import type { Section } from '@/generated/Section';

const ACTION_OPTIONS = [
  { label: 'Move to', value: 'move_to_section' },
  { label: 'Close', value: 'close_task' },
  { label: 'Open', value: 'open_task' },
  { label: 'Set priority', value: 'set_priority' },
] as const;

const PRIORITY_OPTIONS: readonly { label: string; value: Priority }[] = [
  { label: 'Critical', value: 'critical' },
  { label: 'Elevated', value: 'elevated' },
  { label: 'Normal', value: 'normal' },
];

function isPriority(value: string): value is Priority {
  return PRIORITY_OPTIONS.some((option) => option.value === value);
}

export type ActionEditorProps = {
  action: AutomationAction;
  sections: readonly Section[];
  label: string;
  onChange: (action: AutomationAction) => void;
};

export function ActionEditor(props: ActionEditorProps) {
  const { action, sections, label, onChange } = props;

  const kind = action.kind;
  const moveSectionId =
    action.kind === 'move_to_section' ? action.section_id : (sections[0]?.id ?? null);
  const priority = action.kind === 'set_priority' ? action.priority : 'normal';

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
        const nextPriority = action.kind === 'set_priority' ? action.priority : 'normal';
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
        className="md:w-40 [&_select]:min-h-11 md:[&_select]:min-h-0"
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
          className="md:w-36 [&_select]:min-h-11 md:[&_select]:min-h-0"
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
          className="md:w-36 [&_select]:min-h-11 md:[&_select]:min-h-0"
          aria-label={`Priority for ${label}`}
          value={priority}
          onChange={(event) => {
            const { value } = event.target;
            if (!isPriority(value)) {
              return;
            }
            onChange({ kind: 'set_priority', priority: value });
          }}
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
