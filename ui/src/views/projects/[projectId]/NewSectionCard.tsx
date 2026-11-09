import { Button, Card, Modal, ModalBody, ModalFooter, ModalHeader } from 'flowbite-react';
import type { CardTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { useState } from 'react';
import { HiPlusCircle } from 'react-icons/hi2';
import { z } from 'zod';
import { useAppForm } from '@/hooks/useAppForm';

const schema = z.object({
  name: z.string().trim().min(1, 'A section needs a name.'),
});

const cardTheme: DeepPartial<CardTheme> = {
  root: {
    base: 'flex h-full rounded-lg border-2 border-dashed border-line-strong bg-transparent shadow-none',
    children: 'flex h-full flex-col p-0',
  },
};

export type NewSectionCardProps = {
  onAdd: (name: string) => void | Promise<void>;
};

export function NewSectionCard(props: NewSectionCardProps) {
  const { onAdd } = props;

  const [open, setOpen] = useState(false);

  const form = useAppForm({
    defaultValues: { name: '' },
    validators: { onChange: schema, onMount: schema },
    onSubmit: async ({ value }) => {
      await onAdd(value.name.trim());
      form.reset();
      void form.validate('change');
      setOpen(false);
    },
  });

  function close() {
    form.reset();
    void form.validate('change');
    setOpen(false);
  }

  return (
    <>
      <Card
        theme={cardTheme}
        className="h-32 w-full shrink-0 bg-transparent md:h-full md:w-96 dark:bg-transparent"
      >
        <button
          type="button"
          onClick={() => setOpen(true)}
          className="text-muted flex h-full w-full flex-col items-center justify-center gap-2 rounded-lg bg-transparent transition-colors hover:text-zinc-200 dark:bg-transparent"
        >
          <HiPlusCircle aria-hidden="true" className="h-8 w-8" />
          <span className="text-base font-normal italic">Add section</span>
        </button>
      </Card>
      <Modal show={open} onClose={close}>
        <ModalHeader>Add section</ModalHeader>
        <ModalBody>
          <form
            id="new-section-form"
            onSubmit={(event) => {
              event.preventDefault();
              void form.handleSubmit();
            }}
            className="flex flex-col gap-2"
          >
            <form.AppField name="name">
              {(field) => (
                <field.FormTextField
                  label="Section name"
                  placeholder="Section name"
                  sizing="sm"
                  autoFocus
                />
              )}
            </form.AppField>
          </form>
        </ModalBody>
        <ModalFooter className="justify-between">
          <Button color="alternative" onClick={close}>
            Cancel
          </Button>
          <form.Subscribe
            selector={(state) => ({
              isValid: state.isValid,
              submitting: state.isSubmitting,
            })}
          >
            {({ isValid, submitting }) => (
              <Button type="submit" form="new-section-form" disabled={submitting || !isValid}>
                Add
              </Button>
            )}
          </form.Subscribe>
        </ModalFooter>
      </Modal>
    </>
  );
}
