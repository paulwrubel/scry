import { Button } from 'flowbite-react';
import { useEffect, useRef, useState } from 'react';
import { z } from 'zod';
import { useAppForm } from '@/hooks/useAppForm';

const schema = z.object({
  title: z.string().trim().min(1, 'A task needs a title.'),
});

type NewTaskFormProps = {
  onCreate: (title: string) => Promise<void>;
};

export function NewTaskForm(props: NewTaskFormProps) {
  const { onCreate } = props;

  const [open, setOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const formRef = useRef<HTMLFormElement>(null);

  const form = useAppForm({
    defaultValues: { title: '' },
    validators: { onChange: schema },
    onSubmit: async ({ value }) => {
      const submitted = value.title;
      await onCreate(submitted.trim());
      if (form.getFieldValue('title') === submitted) {
        form.reset();
      }
      inputRef.current?.focus();
    },
  });

  function cancel() {
    form.reset();
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
      <form.AppField name="title">
        {(field) => (
          <field.FormTextField
            label="Task title"
            hideLabel
            placeholder="Task title"
            sizing="sm"
            autoFocus
            inputRef={inputRef}
          />
        )}
      </form.AppField>
      <form.Subscribe
        selector={(state) => ({
          title: state.values.title,
          submitting: state.isSubmitting,
        })}
      >
        {({ title, submitting }) => (
          <div className="flex justify-between gap-2">
            <Button type="button" size="xs" color="alternative" onClick={cancel}>
              Cancel
            </Button>
            <Button type="submit" size="xs" disabled={submitting || title.trim().length === 0}>
              Add
            </Button>
          </div>
        )}
      </form.Subscribe>
    </form>
  );
}
