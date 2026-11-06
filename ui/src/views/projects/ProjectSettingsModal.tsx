import { Button, Modal, ModalBody, ModalFooter, ModalHeader } from 'flowbite-react';
import { useState } from 'react';
import { z } from 'zod';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { TaskSortingMode } from '@/generated/TaskSortingMode';
import { useAppForm } from '@/hooks/useAppForm';
import { useProjectMutations } from '@/hooks/useProjectMutations';

const SORTING_MODES: { label: string; value: TaskSortingMode }[] = [
  { label: 'Alphabetical', value: 'alphabetical' },
  { label: 'Alphabetical (case-insensitive)', value: 'alphabetical-case-insensitive' },
  { label: 'Creation order', value: 'created' },
  { label: 'Manual', value: 'manual' },
  { label: 'Priority', value: 'priority' },
];

const schema = z.object({
  name: z.string().trim().min(1, 'A project needs a name.'),
  description: z.string(),
  task_sorting_mode: z.string(),
  show_priority: z.boolean(),
});

export type ProjectSettingsModalProps = {
  project: ProjectSnapshot;
  onClose: () => void;
};

export function ProjectSettingsModal(props: ProjectSettingsModalProps) {
  const { project, onClose } = props;
  const { saveFields } = useProjectMutations(project);

  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const form = useAppForm({
    defaultValues: {
      name: project.name,
      description: project.description ?? '',
      task_sorting_mode: String(project.task_sorting_mode),
      show_priority: project.show_priority,
    },
    validators: { onChange: schema },
    onSubmit: async ({ value }) => {
      setSaving(true);
      setError(null);

      try {
        await saveFields({
          name: value.name,
          description: value.description,
          task_sorting_mode: value.task_sorting_mode as TaskSortingMode,
          show_priority: value.show_priority,
        });
        onClose();
      } catch {
        setError('Could not save this project.');
      } finally {
        setSaving(false);
      }
    },
  });

  return (
    <Modal show onClose={onClose}>
      <ModalHeader>Project settings</ModalHeader>
      <ModalBody>
        <form
          id="project-settings-form"
          onSubmit={(event) => {
            event.preventDefault();
            void form.handleSubmit();
          }}
          className="flex flex-col gap-4 text-zinc-300"
        >
          <form.AppField name="name">
            {(field) => <field.FormTextField label="Name" />}
          </form.AppField>
          <form.AppField name="description">
            {(field) => (
              <field.FormTextAreaField label="Description" placeholder="Optional description" />
            )}
          </form.AppField>
          <form.AppField name="task_sorting_mode">
            {(field) => <field.SelectControl label="Sorting" items={SORTING_MODES} />}
          </form.AppField>
          <form.AppField name="show_priority">
            {(field) => <field.ToggleControl label="Show priority" />}
          </form.AppField>
          {error === null ? null : (
            <p className="text-danger text-sm" role="alert">
              {error}
            </p>
          )}
        </form>
      </ModalBody>
      <ModalFooter>
        <Button color="default" onClick={onClose}>
          Cancel
        </Button>
        <Button type="submit" form="project-settings-form" disabled={saving || !form.state.isValid}>
          {saving ? 'Saving...' : 'Save'}
        </Button>
      </ModalFooter>
    </Modal>
  );
}
