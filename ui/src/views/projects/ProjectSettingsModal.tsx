import { Button, Modal, ModalBody, ModalFooter, ModalHeader, ToggleSwitch } from 'flowbite-react';
import { useState } from 'react';
import { SelectControl } from '@/components/form-controls/SelectControl';
import { TextAreaControl } from '@/components/form-controls/TextAreaControl';
import { TextInputControl } from '@/components/form-controls/TextInputControl';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { TaskSortingMode } from '@/generated/TaskSortingMode';
import { useProjectMutations } from '@/hooks/useProjectMutations';

const SORTING_MODES: { label: string; value: TaskSortingMode }[] = [
  { label: 'Alphabetical', value: 'alphabetical' },
  { label: 'Alphabetical (case-insensitive)', value: 'alphabetical-case-insensitive' },
  { label: 'Creation order', value: 'created' },
  { label: 'Manual', value: 'manual' },
  { label: 'Priority', value: 'priority' },
];

export type ProjectSettingsModalProps = {
  project: ProjectSnapshot;
  onClose: () => void;
};

export function ProjectSettingsModal(props: ProjectSettingsModalProps) {
  const { project, onClose } = props;
  const { saveFields } = useProjectMutations(project);

  const [name, setName] = useState(project.name);
  const [description, setDescription] = useState(project.description ?? '');
  const [sorting, setSorting] = useState<TaskSortingMode>(project.task_sorting_mode);
  const [showPriority, setShowPriority] = useState(project.show_priority);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (name.trim().length === 0) {
      setError('A project needs a name.');
      return;
    }

    setSaving(true);
    setError(null);

    try {
      await saveFields({
        name,
        description,
        task_sorting_mode: sorting,
        show_priority: showPriority,
      });
      onClose();
    } catch {
      setError('Could not save this project.');
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal show onClose={onClose}>
      <ModalHeader>Project settings</ModalHeader>
      <ModalBody>
        <form
          id="project-settings-form"
          onSubmit={handleSubmit}
          className="flex flex-col gap-4 text-zinc-300"
        >
          <TextInputControl label="Name" value={name} onChange={setName} />
          <TextAreaControl
            label="Description"
            value={description}
            onChange={setDescription}
            placeholder="Optional description"
          />
          <SelectControl
            label="Sorting"
            value={sorting}
            items={SORTING_MODES}
            onChange={(value) => setSorting(value as TaskSortingMode)}
          />
          <ToggleSwitch checked={showPriority} onChange={setShowPriority} label="Show priority" />
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
        <Button type="submit" form="project-settings-form" disabled={saving}>
          {saving ? 'Saving...' : 'Save'}
        </Button>
      </ModalFooter>
    </Modal>
  );
}
