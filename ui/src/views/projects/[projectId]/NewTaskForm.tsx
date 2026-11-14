import { Button } from 'flowbite-react';
import { useEffect, useRef, useState } from 'react';
import { z } from 'zod';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import { useAppForm } from '@/hooks/useAppForm';
import { parseTaskInput } from '@/utils/tasks/input';

const schema = z.object({
  task: z
    .string()
    .trim()
    .min(1, 'Enter a task title or GitHub URL.')
    .refine(
      (value) => parseTaskInput(value) !== null,
      'Only GitHub issue or pull request URLs are supported.',
    ),
});

export type NewTaskFormProps = {
  onCreate: (title: string, externalIdentity: ExternalIdentity | null) => Promise<void>;
};

export function NewTaskForm(props: NewTaskFormProps) {
  const { onCreate } = props;

  const [open, setOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const formRef = useRef<HTMLFormElement>(null);

  const form = useAppForm({
    defaultValues: { task: '' },
    validators: { onChange: schema, onMount: schema },
    onSubmit: async ({ value }) => {
      const submitted = value.task;
      const parsed = parseTaskInput(value.task);
      if (parsed === null) {
        return;
      }
      await onCreate(parsed.title, parsed.externalIdentity);
      if (form.getFieldValue('task') === submitted) {
        form.reset();
        void form.validate('change');
      }
      inputRef.current?.focus();
    },
  });

  function cancel() {
    form.reset();
    void form.validate('change');
    setOpen(false);
  }

  useEffect(() => {
    if (!open) {
      return;
    }

    function handlePointerDown(event: PointerEvent) {
      if (event.target instanceof Node && formRef.current?.contains(event.target)) {
        return;
      }
      form.reset();
      void form.validate('change');
      setOpen(false);
    }

    document.addEventListener('pointerdown', handlePointerDown);
    return () => document.removeEventListener('pointerdown', handlePointerDown);
  }, [open, form]);

  if (!open) {
    return (
      <Button size="xs" color="alternative" className="shrink-0" onClick={() => setOpen(true)}>
        Add task
      </Button>
    );
  }

  function handleKeyDown(event: React.KeyboardEvent<HTMLFormElement>) {
    if (event.key === 'Escape') {
      event.preventDefault();
      cancel();
    }
  }

  return (
    <form
      ref={formRef}
      onSubmit={(event) => {
        event.preventDefault();
        void form.handleSubmit();
      }}
      onKeyDown={handleKeyDown}
      className="flex shrink-0 flex-col gap-2"
    >
      <form.AppField name="task">
        {(field) => (
          <field.FormTextField
            label="Task title or GitHub URL"
            hideLabel
            placeholder="Task title or GitHub URL"
            sizing="sm"
            autoFocus
            inputRef={inputRef}
          />
        )}
      </form.AppField>
      <form.Subscribe
        selector={(state) => ({
          isValid: state.isValid,
          submitting: state.isSubmitting,
        })}
      >
        {({ isValid, submitting }) => (
          <div className="flex justify-between gap-2">
            <Button type="button" size="xs" color="alternative" onClick={cancel}>
              Cancel
            </Button>
            <Button type="submit" size="xs" disabled={submitting || !isValid}>
              Add
            </Button>
          </div>
        )}
      </form.Subscribe>
    </form>
  );
}
