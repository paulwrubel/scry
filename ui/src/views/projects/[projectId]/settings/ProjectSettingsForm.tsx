import { Button } from 'flowbite-react';
import { useState } from 'react';
import { z } from 'zod';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { useAppForm } from '@/hooks/useAppForm';
import { useProjectMutations } from '@/hooks/useProjectMutations';

const schema = z.object({
  name: z.string().trim().min(1, 'A project needs a name.'),
  description: z.string(),
});

export type ProjectSettingsFormProps = {
  project: ProjectSnapshot;
};

export function ProjectSettingsForm(props: ProjectSettingsFormProps) {
  const { project } = props;
  const { saveFields } = useProjectMutations(project);

  const [error, setError] = useState<string | null>(null);

  const form = useAppForm({
    defaultValues: {
      name: project.name,
      description: project.description ?? '',
    },
    validators: { onChange: schema, onMount: schema },
    listeners: { onChange: () => setError(null) },
    onSubmit: async ({ value }) => {
      setError(null);

      try {
        await saveFields({
          name: value.name,
          description: value.description,
          task_sorting_mode: project.task_sorting_mode,
          show_priority: project.show_priority,
        });
      } catch {
        setError('Could not save this project.');
      }
    },
  });

  return (
    <form
      id="project-settings-form"
      className="flex flex-col gap-4 text-zinc-300"
      onSubmit={(event) => {
        event.preventDefault();
        void form.handleSubmit();
      }}
    >
      <form.AppField name="name">{(field) => <field.FormTextField label="Name" />}</form.AppField>
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
      <div className="flex justify-end">
        <form.Subscribe
          selector={(state) => ({
            isValid: state.isValid,
            submitting: state.isSubmitting,
          })}
        >
          {({ isValid, submitting }) => (
            <Button type="submit" disabled={submitting || error !== null || !isValid}>
              {submitting ? 'Saving...' : 'Save'}
            </Button>
          )}
        </form.Subscribe>
      </div>
    </form>
  );
}
