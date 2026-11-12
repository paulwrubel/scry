import { Button } from 'flowbite-react';
import { useState } from 'react';
import type { Task } from '@/generated/Task';
import { useAppForm } from '@/hooks/useAppForm';
import { FieldHeader } from './FieldHeader';

const INPUT_CLASS =
  'border-line-strong bg-surface text-zinc-200 focus:border-line-hover w-full rounded-lg border p-2 text-sm focus:outline-none';

function resizeTextarea(element: HTMLTextAreaElement | null) {
  if (element === null) {
    return;
  }
  element.style.height = 'auto';
  element.style.height = `${element.scrollHeight}px`;
}

export type DescriptionFieldProps = {
  task: Task;
  readOnly: boolean;
  onSave: (description: string) => Promise<void>;
};

export function DescriptionField(props: DescriptionFieldProps) {
  const { task, readOnly, onSave } = props;

  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const description = task.description?.trim() ?? '';

  const form = useAppForm({
    defaultValues: { description: task.description ?? '' },
    onSubmit: async ({ value }) => {
      setSaving(true);
      setError(null);
      try {
        await onSave(value.description);
        setEditing(false);
      } catch {
        setError('Could not save the description.');
      } finally {
        setSaving(false);
      }
    },
  });

  function startEditing() {
    form.setFieldValue('description', task.description ?? '');
    setError(null);
    setEditing(true);
  }

  function cancelEditing() {
    form.reset();
    setError(null);
    setEditing(false);
  }

  return (
    <div className="flex flex-col gap-1">
      <FieldHeader label="Description" onEdit={editing || readOnly ? undefined : startEditing} />
      {editing ? (
        <form
          className="flex flex-col gap-1"
          onSubmit={(event) => {
            event.preventDefault();
            void form.handleSubmit();
          }}
        >
          <form.AppField name="description">
            {(field) => (
              <textarea
                ref={resizeTextarea}
                aria-label="Description"
                value={field.state.value}
                onChange={(event) => {
                  field.handleChange(event.target.value);
                  resizeTextarea(event.currentTarget);
                }}
                className={`${INPUT_CLASS} resize-none overflow-hidden`}
              />
            )}
          </form.AppField>
          <div className="flex items-center justify-between gap-2">
            <Button
              size="xs"
              color="alternative"
              aria-label="Cancel description edit"
              disabled={saving}
              onClick={cancelEditing}
            >
              Cancel
            </Button>
            <Button
              size="xs"
              aria-label="Save description"
              disabled={saving}
              onClick={() => void form.handleSubmit()}
            >
              {saving ? 'Saving...' : 'Save'}
            </Button>
          </div>
        </form>
      ) : description.length > 0 ? (
        <p className="text-sm whitespace-pre-wrap">{description}</p>
      ) : (
        <p className="text-faint text-sm">No description.</p>
      )}
      {error === null ? null : (
        <p className="text-danger text-sm" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
