import { Button, Modal, ModalBody, ModalFooter, ModalHeader } from 'flowbite-react';
import { useState } from 'react';
import { z } from 'zod';
import { useAppForm } from '@/hooks/useAppForm';
import { useCloseRequest } from '@/hooks/useCloseRequest';
import { useEnqueue } from '@/hooks/useEnqueue';
import { createProjectOperation } from '@/utils/projects/operations';

function schemaFor(existingNames: readonly string[]) {
  return z.object({
    name: z
      .string()
      .trim()
      .min(1, 'A project needs a name.')
      .refine((name) => !existingNames.includes(name), 'A project with this name already exists.'),
    description: z.string(),
  });
}

export type NewProjectModalProps = {
  existingNames: readonly string[];
  onClose: () => void;
  onCreated: (projectId: string) => void;
};

export function NewProjectModal(props: NewProjectModalProps) {
  const { existingNames, onClose, onCreated } = props;

  const enqueue = useEnqueue();

  const [error, setError] = useState<string | null>(null);

  const form = useAppForm({
    defaultValues: {
      name: '',
      description: '',
    },
    validators: { onChange: schemaFor(existingNames), onMount: schemaFor(existingNames) },
    listeners: { onChange: () => setError(null) },
    onSubmit: async ({ value }) => {
      setError(null);

      try {
        const operation = createProjectOperation(value);
        await enqueue(operation);
        onCreated(operation.id);
      } catch {
        setError('Could not create this project.');
      }
    },
  });

  // This modal is mounted only while open.
  useCloseRequest(true, onClose);

  return (
    <Modal show onClose={onClose}>
      <ModalHeader>New project</ModalHeader>
      <ModalBody>
        <form
          id="new-project-form"
          onSubmit={(event) => {
            event.preventDefault();
            void form.handleSubmit();
          }}
          className="flex flex-col gap-4 text-zinc-300"
        >
          <form.AppField name="name">
            {(field) => <field.FormTextField label="Name" placeholder="Project name" />}
          </form.AppField>
          <form.AppField name="description">
            {(field) => (
              <field.FormTextAreaField label="Description" placeholder="Optional description" />
            )}
          </form.AppField>
          {error === null ? null : (
            <p className="text-danger text-sm" role="alert">
              {error}
            </p>
          )}
        </form>
      </ModalBody>
      <ModalFooter className="justify-between">
        <Button color="alternative" onClick={onClose}>
          Cancel
        </Button>
        <form.Subscribe
          selector={(state) => ({
            isValid: state.isValid,
            submitting: state.isSubmitting,
          })}
        >
          {({ isValid, submitting }) => (
            <Button
              type="submit"
              form="new-project-form"
              disabled={submitting || error !== null || !isValid}
            >
              {submitting ? 'Creating...' : 'Create'}
            </Button>
          )}
        </form.Subscribe>
      </ModalFooter>
    </Modal>
  );
}
