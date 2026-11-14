import { Badge, Button } from 'flowbite-react';
import { useState } from 'react';
import type { Task } from '@/generated/Task';
import { useAppForm } from '@/hooks/useAppForm';
import { tagBadgeStyle } from '@/utils/tasks/tagColor';
import { FieldHeader } from './FieldHeader';

const INPUT_CLASS =
  'border-line-strong bg-surface text-zinc-200 focus:border-line-hover w-full rounded-lg border p-2 text-sm focus:outline-none';

export type TagsFieldProps = {
  task: Task;
  onSave: (tags: string[]) => Promise<void>;
};

export function TagsField(props: TagsFieldProps) {
  const { task, onSave } = props;

  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const form = useAppForm({
    defaultValues: { tags: task.tags.join(', ') },
    onSubmit: async ({ value }) => {
      setSaving(true);
      setError(null);
      try {
        await onSave(
          value.tags
            .split(',')
            .map((tag) => tag.trim())
            .filter((tag) => tag.length > 0),
        );
        setEditing(false);
      } catch {
        setError('Could not save the tags.');
      } finally {
        setSaving(false);
      }
    },
  });

  function startEditing() {
    form.setFieldValue('tags', task.tags.join(', '));
    setError(null);
    setEditing(true);
  }

  function cancelEditing() {
    form.reset();
    setError(null);
    setEditing(false);
  }

  return (
    <div className="flex flex-col gap-1">
      <FieldHeader label="Tags" onEdit={editing ? undefined : startEditing} />
      {editing ? (
        <form
          className="flex flex-col gap-1"
          onSubmit={(event) => {
            event.preventDefault();
            void form.handleSubmit();
          }}
        >
          <form.AppField name="tags">
            {(field) => (
              <input
                aria-label="Tags"
                value={field.state.value}
                placeholder="comma, separated"
                onChange={(event) => field.handleChange(event.target.value)}
                className={INPUT_CLASS}
              />
            )}
          </form.AppField>
          <div className="flex items-center justify-between gap-2">
            <Button
              size="xs"
              color="alternative"
              aria-label="Cancel tags edit"
              disabled={saving}
              onClick={cancelEditing}
            >
              Cancel
            </Button>
            <Button
              size="xs"
              aria-label="Save tags"
              disabled={saving}
              onClick={() => void form.handleSubmit()}
            >
              {saving ? 'Saving...' : 'Save'}
            </Button>
          </div>
        </form>
      ) : task.tags.length === 0 ? (
        <p className="text-faint text-sm">No tags</p>
      ) : (
        <div className="flex flex-wrap items-center gap-1">
          {task.tags.map((tag) => (
            <Badge key={tag} color="gray" style={tagBadgeStyle(tag)}>
              {tag}
            </Badge>
          ))}
        </div>
      )}
      {error === null ? null : (
        <p className="text-danger text-sm" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
