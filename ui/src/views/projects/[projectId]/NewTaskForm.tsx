import { Button } from 'flowbite-react';
import { useEffect, useRef } from 'react';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import { useCreateTaskForm } from '@/hooks/useCreateTaskForm';

export type NewTaskFormProps = {
  onCreate: (title: string, externalIdentity: ExternalIdentity | null) => Promise<void>;
};

export function NewTaskForm(props: NewTaskFormProps) {
  const { onCreate } = props;

  const { form, open, inputRef, openForm, closeForm } = useCreateTaskForm(onCreate);
  const formRef = useRef<HTMLFormElement>(null);

  useEffect(() => {
    if (!open) {
      return;
    }

    function handlePointerDown(event: PointerEvent) {
      if (event.target instanceof Node && formRef.current?.contains(event.target)) {
        return;
      }
      closeForm();
    }

    document.addEventListener('pointerdown', handlePointerDown);
    return () => document.removeEventListener('pointerdown', handlePointerDown);
  }, [open, closeForm]);

  if (!open) {
    return (
      <Button size="xs" color="alternative" className="shrink-0" onClick={openForm}>
        Add task
      </Button>
    );
  }

  function handleKeyDown(event: React.KeyboardEvent<HTMLFormElement>) {
    if (event.key === 'Escape') {
      event.preventDefault();
      closeForm();
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
            <Button type="button" size="xs" color="alternative" onClick={closeForm}>
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
