import { Badge, Button } from 'flowbite-react';
import { useState } from 'react';
import { z } from 'zod';
import type { Priority } from '@/generated/Priority';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { useAppForm } from '@/hooks/useAppForm';
import type { TaskFieldPatch } from '@/utils/taskOperations';

const PRIORITIES: { label: string; value: Priority }[] = [
  { label: 'Critical', value: 'critical' },
  { label: 'High', value: 'high' },
  { label: 'Medium', value: 'medium' },
  { label: 'Low', value: 'low' },
  { label: 'Minimal', value: 'minimal' },
];

const schema = z.object({
  title: z.string().trim().min(1, 'A task needs a title.'),
  description: z.string(),
  priority: z.string(),
  sectionId: z.string(),
  tags: z.string(),
});

export type TaskFieldsFormProps = {
  task: Task;
  project: ProjectSnapshot;
  onSave: (patch: TaskFieldPatch, sectionId: string | null) => Promise<void>;
};

export function TaskFieldsForm(props: TaskFieldsFormProps) {
  const { task, project, onSave } = props;

  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const sections = [
    { label: 'No section', value: '' },
    ...project.sections.map((section) => ({ label: section.name, value: section.id })),
  ];

  const form = useAppForm({
    defaultValues: {
      title: task.title,
      description: task.description ?? '',
      priority: String(task.priority),
      sectionId: task.section_id ?? '',
      tags: task.tags.join(', '),
    },
    validators: { onChange: schema },
    onSubmit: async ({ value }) => {
      setSaving(true);
      setError(null);
      setSaved(false);

      try {
        await onSave(
          {
            title: value.title,
            description: value.description,
            priority: value.priority as Priority,
            tags: value.tags.split(','),
          },
          value.sectionId.length === 0 ? null : value.sectionId,
        );
        setSaved(true);
      } catch {
        setError('Could not save this task.');
      } finally {
        setSaving(false);
      }
    },
  });

  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        void form.handleSubmit();
      }}
      className="flex flex-col gap-4"
    >
      <form.AppField name="title">{(field) => <field.FormTextField label="Title" />}</form.AppField>
      <form.AppField name="description">
        {(field) => (
          <field.FormTextAreaField label="Description" placeholder="Optional description" />
        )}
      </form.AppField>
      <form.AppField name="priority">
        {(field) => <field.SelectControl label="Priority" items={PRIORITIES} />}
      </form.AppField>
      <form.AppField name="sectionId">
        {(field) => <field.SelectControl label="Section" items={sections} />}
      </form.AppField>
      <form.AppField name="tags">
        {(field) => <field.FormTextField label="Tags" placeholder="comma, separated" />}
      </form.AppField>
      {error === null ? null : (
        <p className="text-danger text-sm" role="alert">
          {error}
        </p>
      )}
      <div className="flex items-center gap-3">
        <Button type="submit" disabled={saving || !form.state.isValid}>
          {saving ? 'Saving...' : 'Save'}
        </Button>
        {saved ? <Badge color="success">Saved locally</Badge> : null}
      </div>
    </form>
  );
}
