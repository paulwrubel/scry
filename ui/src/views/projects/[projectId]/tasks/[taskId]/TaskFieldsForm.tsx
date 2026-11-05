import { Badge, Button } from 'flowbite-react';
import { useState } from 'react';
import type { Priority } from '@/generated/Priority';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import type { TaskFieldPatch } from '@/utils/taskOperations';
import { SelectControl } from '@/components/form-controls/SelectControl';
import { TextAreaControl } from '@/components/form-controls/TextAreaControl';
import { TextInputControl } from '@/components/form-controls/TextInputControl';

const PRIORITIES: { label: string; value: Priority }[] = [
  { label: 'Critical', value: 'critical' },
  { label: 'High', value: 'high' },
  { label: 'Medium', value: 'medium' },
  { label: 'Low', value: 'low' },
  { label: 'Minimal', value: 'minimal' },
];

export type TaskFieldsFormProps = {
  task: Task;
  project: ProjectSnapshot;
  onSave: (patch: TaskFieldPatch, sectionId: string | null) => Promise<void>;
};

export function TaskFieldsForm(props: TaskFieldsFormProps) {
  const { task, project, onSave } = props;

  const [title, setTitle] = useState(task.title);
  const [description, setDescription] = useState(task.description ?? '');
  const [priority, setPriority] = useState<Priority>(task.priority);
  const [tags, setTags] = useState(task.tags.join(', '));
  const [sectionId, setSectionId] = useState(task.section_id ?? '');
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [saving, setSaving] = useState(false);

  const sections = [
    { label: 'No section', value: '' },
    ...project.sections.map((section) => ({ label: section.name, value: section.id })),
  ];

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (title.trim().length === 0) {
      setError('A task needs a title.');
      setSaved(false);
      return;
    }

    setSaving(true);
    setError(null);
    setSaved(false);

    try {
      await onSave(
        {
          title,
          description,
          priority,
          tags: tags.split(','),
        },
        sectionId.length === 0 ? null : sectionId,
      );
      setSaved(true);
    } catch {
      setError('Could not save this task.');
    } finally {
      setSaving(false);
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <TextInputControl label="Title" value={title} onChange={setTitle} />
      <TextAreaControl
        label="Description"
        value={description}
        onChange={setDescription}
        placeholder="Optional description"
      />
      <SelectControl
        label="Priority"
        value={priority}
        items={PRIORITIES}
        onChange={(value) => setPriority(value as Priority)}
      />
      <SelectControl label="Section" value={sectionId} items={sections} onChange={setSectionId} />
      <TextInputControl
        label="Tags"
        value={tags}
        onChange={setTags}
        placeholder="comma, separated"
      />
      {error === null ? null : (
        <p className="text-sm text-red-400" role="alert">
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
  );
}
