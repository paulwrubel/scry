import { Badge, Button, Card, ToggleSwitch } from 'flowbite-react';
import { useState } from 'react';
import { SelectControl } from '@/components/form-controls/SelectControl';
import { TextAreaControl } from '@/components/form-controls/TextAreaControl';
import { TextInputControl } from '@/components/form-controls/TextInputControl';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { TaskSortingMode } from '@/generated/TaskSortingMode';
import { useProjectMutations } from '@/hooks/useProjectMutations';
import { useActiveProject } from '@/providers/ActiveProject';
import { useReplicaSnapshot } from '@/providers/Replica';

const SORTING_MODES: { label: string; value: TaskSortingMode }[] = [
  { label: 'Alphabetical', value: 'alphabetical' },
  { label: 'Alphabetical (case-insensitive)', value: 'alphabetical-case-insensitive' },
  { label: 'Creation order', value: 'created' },
  { label: 'Manual', value: 'manual' },
  { label: 'Priority', value: 'priority' },
];

export function ProjectSection() {
  const { activeProjectId } = useActiveProject();
  const snapshot = useReplicaSnapshot();
  const project = snapshot.visible.projects.find((candidate) => candidate.id === activeProjectId);

  if (project === undefined) {
    return (
      <Card>
        <h2 className="text-lg font-semibold">Project</h2>
        <p className="text-muted text-sm">No active project.</p>
      </Card>
    );
  }

  return <ProjectForm key={project.id} project={project} />;
}

function ProjectForm(props: { project: ProjectSnapshot }) {
  const { project } = props;
  const { saveFields } = useProjectMutations(project);

  const [name, setName] = useState(project.name);
  const [description, setDescription] = useState(project.description ?? '');
  const [sorting, setSorting] = useState<TaskSortingMode>(project.task_sorting_mode);
  const [showPriority, setShowPriority] = useState(project.show_priority);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [saving, setSaving] = useState(false);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (name.trim().length === 0) {
      setError('A project needs a name.');
      setSaved(false);
      return;
    }

    setSaving(true);
    setError(null);
    setSaved(false);

    try {
      await saveFields({
        name,
        description,
        task_sorting_mode: sorting,
        show_priority: showPriority,
      });
      setSaved(true);
    } catch {
      setError('Could not save this project.');
    } finally {
      setSaving(false);
    }
  }

  return (
    <Card>
      <h2 className="text-lg font-semibold">Project</h2>
      <form onSubmit={handleSubmit} className="flex flex-col gap-4">
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
        <div className="flex items-center gap-3">
          <Button type="submit" disabled={saving}>
            {saving ? 'Saving...' : 'Save'}
          </Button>
          {saved ? <Badge color="success">Saved locally</Badge> : null}
        </div>
      </form>
    </Card>
  );
}
