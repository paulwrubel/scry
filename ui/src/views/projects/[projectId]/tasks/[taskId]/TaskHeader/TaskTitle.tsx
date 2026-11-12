import { Button, TextInput } from 'flowbite-react';
import { useState } from 'react';
import { HiCheck, HiPencil, HiXMark } from 'react-icons/hi2';
import { z } from 'zod';
import { fieldErrorMessage } from '@/components/form-controls/fieldErrors';
import { useAppForm } from '@/hooks/useAppForm';

const ICON_BUTTON_CLASS =
  'text-muted inline-flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center border-0 bg-transparent p-0 transition-colors hover:text-zinc-200 focus:outline-none focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50';

const schema = z.object({
  title: z.string().trim().min(1, 'A task needs a title.'),
});

export type TaskTitleProps = {
  title: string;
  readOnly: boolean;
  onSave: (title: string) => Promise<void>;
};

export function TaskTitle(props: TaskTitleProps) {
  const { title, readOnly, onSave } = props;

  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const form = useAppForm({
    defaultValues: { title },
    validators: { onChange: schema, onMount: schema },
    onSubmit: async ({ value }) => {
      const trimmed = value.title.trim();
      if (trimmed === title) {
        setEditing(false);
        return;
      }

      setSaving(true);
      setError(null);
      try {
        await onSave(trimmed);
        setEditing(false);
      } catch {
        setError('Could not save the title.');
      } finally {
        setSaving(false);
      }
    },
  });

  function startEditing() {
    form.reset();
    form.setFieldValue('title', title);
    setError(null);
    setEditing(true);
  }

  function cancelEditing() {
    form.reset();
    setError(null);
    setEditing(false);
  }

  if (editing) {
    return (
      <form
        className="flex w-full min-w-0 flex-col gap-1"
        onSubmit={(event) => {
          event.preventDefault();
          void form.handleSubmit();
        }}
        onKeyDown={(event) => {
          if (event.key === 'Escape') {
            event.preventDefault();
            cancelEditing();
          }
        }}
      >
        <form.AppField name="title">
          {(field) => {
            const titleError = field.state.meta.isTouched
              ? fieldErrorMessage(field.state.meta.errors)
              : null;

            return (
              <div className="flex w-full min-w-0 flex-col gap-1">
                <div className="flex min-w-0 items-center gap-2">
                  <TextInput
                    aria-label="Title"
                    autoFocus
                    sizing="lg"
                    theme={{ field: { input: { sizes: { lg: 'p-2 text-2xl' } } } }}
                    value={field.state.value}
                    onChange={(event) => field.handleChange(event.target.value)}
                    className="min-w-0 flex-1"
                  />
                  <Button
                    size="sm"
                    color="gray"
                    outline
                    aria-label="Save title"
                    title="Save title"
                    disabled={saving}
                    onClick={() => void form.handleSubmit()}
                  >
                    <HiCheck aria-hidden="true" className="h-5 w-5" />
                  </Button>
                  <Button
                    size="sm"
                    color="gray"
                    outline
                    aria-label="Cancel title edit"
                    title="Cancel title edit"
                    disabled={saving}
                    onClick={cancelEditing}
                  >
                    <HiXMark aria-hidden="true" className="h-5 w-5" />
                  </Button>
                </div>
                {titleError === null ? null : (
                  <p className="text-danger text-sm" role="alert">
                    {titleError}
                  </p>
                )}
              </div>
            );
          }}
        </form.AppField>
        {error === null ? null : (
          <p className="text-danger text-sm" role="alert">
            {error}
          </p>
        )}
      </form>
    );
  }

  return (
    <div className="flex min-w-0 items-center gap-2">
      <h1 className="min-w-0 text-2xl font-semibold wrap-break-word">{title}</h1>
      {readOnly ? null : (
        <button
          type="button"
          aria-label="Edit title"
          title="Edit title"
          onClick={startEditing}
          className={ICON_BUTTON_CLASS}
        >
          <HiPencil aria-hidden="true" className="h-3.5 w-3.5" />
        </button>
      )}
    </div>
  );
}
