import { Badge, Button } from 'flowbite-react';
import { useState } from 'react';
import { HiPencil } from 'react-icons/hi2';
import type { Task } from '@/generated/Task';
import { tagBadgeStyle } from '@/utils/tagColor';

const INPUT_CLASS =
  'border-line-strong bg-surface text-zinc-200 focus:border-line-hover w-full rounded-lg border p-2 text-sm focus:outline-none';

function resizeTextarea(element: HTMLTextAreaElement | null) {
  if (element === null) {
    return;
  }
  element.style.height = 'auto';
  element.style.height = `${element.scrollHeight}px`;
}

function PencilButton(props: { label: string; onClick: () => void }) {
  const { label, onClick } = props;

  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className="text-muted inline-flex h-6 w-6 shrink-0 items-center justify-center border-0 bg-transparent p-0 transition-colors hover:text-zinc-200 focus:outline-none focus-visible:outline-none"
    >
      <HiPencil aria-hidden="true" className="h-3.5 w-3.5" />
    </button>
  );
}

function FieldHeader(props: { label: string; onEdit?: () => void }) {
  const { label, onEdit } = props;

  return (
    <div className="flex items-center justify-between gap-2">
      <span className="text-muted text-sm font-semibold">{label}</span>
      {onEdit === undefined ? null : (
        <PencilButton label={`Edit ${label.toLowerCase()}`} onClick={onEdit} />
      )}
    </div>
  );
}

function DescriptionField(props: { task: Task; onSave: (description: string) => Promise<void> }) {
  const { task, onSave } = props;

  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(task.description ?? '');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const description = task.description?.trim() ?? '';

  function startEditing() {
    setDraft(task.description ?? '');
    setError(null);
    setEditing(true);
  }

  function cancelEditing() {
    setEditing(false);
    setError(null);
  }

  async function save() {
    setSaving(true);
    setError(null);
    try {
      await onSave(draft);
      setEditing(false);
    } catch {
      setError('Could not save the description.');
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="flex flex-col gap-1">
      <FieldHeader label="Description" onEdit={editing ? undefined : startEditing} />
      {editing ? (
        <>
          <textarea
            ref={resizeTextarea}
            aria-label="Description"
            value={draft}
            onChange={(event) => {
              setDraft(event.target.value);
              resizeTextarea(event.currentTarget);
            }}
            className={`${INPUT_CLASS} resize-none overflow-hidden`}
          />
          <div className="flex items-center justify-between gap-2">
            <Button
              size="xs"
              color="alternative"
              aria-label="Cancel description edit"
              disabled={saving}
              onClick={cancelEditing}
            >
              Cancel
            </Button>
            <Button
              size="xs"
              aria-label="Save description"
              disabled={saving}
              onClick={() => void save()}
            >
              {saving ? 'Saving...' : 'Save'}
            </Button>
          </div>
        </>
      ) : description.length > 0 ? (
        <p className="text-sm whitespace-pre-wrap">{description}</p>
      ) : (
        <p className="text-faint text-sm">No description.</p>
      )}
      {error === null ? null : (
        <p className="text-danger text-sm" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

function TagsField(props: { task: Task; onSave: (tags: string[]) => Promise<void> }) {
  const { task, onSave } = props;

  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function startEditing() {
    setDraft(task.tags.join(', '));
    setError(null);
    setEditing(true);
  }

  function cancelEditing() {
    setEditing(false);
    setError(null);
  }

  async function save() {
    setSaving(true);
    setError(null);
    try {
      await onSave(
        draft
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
  }

  return (
    <div className="flex flex-col gap-1">
      <FieldHeader label="Tags" onEdit={editing ? undefined : startEditing} />
      {editing ? (
        <>
          <input
            aria-label="Tags"
            value={draft}
            placeholder="comma, separated"
            onChange={(event) => setDraft(event.target.value)}
            className={INPUT_CLASS}
          />
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
            <Button size="xs" aria-label="Save tags" disabled={saving} onClick={() => void save()}>
              {saving ? 'Saving...' : 'Save'}
            </Button>
          </div>
        </>
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

export type TaskFieldsProps = {
  task: Task;
  onSaveDescription: (description: string) => Promise<void>;
  onSaveTags: (tags: string[]) => Promise<void>;
};

export function TaskFields(props: TaskFieldsProps) {
  const { task, onSaveDescription, onSaveTags } = props;

  return (
    <section className="flex flex-col gap-4">
      <DescriptionField task={task} onSave={onSaveDescription} />
      <TagsField task={task} onSave={onSaveTags} />
    </section>
  );
}
