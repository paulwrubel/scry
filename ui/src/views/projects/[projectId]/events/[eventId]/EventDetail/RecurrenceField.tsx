import { Button } from 'flowbite-react';
import { useState } from 'react';
import { defaultRecurrenceDraft, describeDraft, draftFromRrule } from '@/utils/events/recurrence';
import { RecurrenceDialog } from './RecurrenceDialog';

export type RecurrenceFieldProps = {
  value: string;
  startDate: string;
  onChange: (value: string) => void;
  error: string | null;
};

/// The Outlook-style Repeat row. Editing happens in the dialog; the RRULE text
/// is shown only as a read-only preview.
export function RecurrenceField(props: RecurrenceFieldProps) {
  const { value, startDate, onChange, error } = props;

  const [editing, setEditing] = useState(false);

  const draft = value.trim().length === 0 ? null : draftFromRrule(value, startDate);
  const custom = value.trim().length > 0 && draft === null;
  const summary =
    value.trim().length === 0
      ? 'Does not repeat'
      : draft === null
        ? 'Custom recurrence'
        : describeDraft(draft);

  function apply(next: string) {
    onChange(next);
    setEditing(false);
  }

  function remove() {
    onChange('');
    setEditing(false);
  }

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex flex-col items-stretch gap-1.5">
        <span className="text-sm">Repeat</span>
        <div className="flex flex-wrap items-center gap-2">
          <Button
            color="light"
            size="sm"
            type="button"
            onClick={() => setEditing(true)}
            disabled={custom}
          >
            {value.trim().length === 0 ? 'Add recurrence' : 'Edit recurrence'}
          </Button>
          {value.trim().length === 0 ? null : (
            <Button color="alternative" size="sm" type="button" onClick={remove}>
              Clear
            </Button>
          )}
          <span className="text-muted text-sm">{summary}</span>
        </div>
      </div>
      {value.trim().length === 0 ? null : (
        <code className="text-faint overflow-x-auto text-xs">{value}</code>
      )}
      {custom ? (
        <p className="text-muted text-xs">
          This rule was created outside the recurrence controls and can only be cleared here.
        </p>
      ) : null}
      {error === null ? null : (
        <p className="text-danger text-sm" role="alert">
          {error}
        </p>
      )}

      <RecurrenceDialog
        key={`${editing}:${value}:${startDate}`}
        show={editing}
        startDate={startDate}
        initial={draft ?? defaultRecurrenceDraft(startDate)}
        canRemove={value.trim().length > 0}
        onApply={apply}
        onRemove={remove}
        onClose={() => setEditing(false)}
      />
    </div>
  );
}
