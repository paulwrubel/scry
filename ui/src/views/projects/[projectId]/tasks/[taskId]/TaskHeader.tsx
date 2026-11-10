import { Button, TextInput } from 'flowbite-react';
import { useState } from 'react';
import { HiCheck, HiPencil, HiXMark } from 'react-icons/hi2';
import { Link, useLocation } from 'react-router-dom';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';

const ICON_BUTTON_CLASS =
  'text-muted inline-flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center border-0 bg-transparent p-0 transition-colors hover:text-zinc-200 focus:outline-none focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50';

function TaskTitle(props: { title: string; onSave: (title: string) => Promise<void> }) {
  const { title, onSave } = props;

  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(title);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function startEditing() {
    setDraft(title);
    setError(null);
    setEditing(true);
  }

  function cancelEditing() {
    setEditing(false);
    setError(null);
  }

  async function save() {
    const trimmed = draft.trim();
    if (trimmed.length === 0) {
      setError('A task needs a title.');
      return;
    }
    if (trimmed === title) {
      setEditing(false);
      return;
    }

    setSaving(true);
    setError(null);
    try {
      await onSave(trimmed);
      setEditing(false);
    } catch {
      setError('Could not save the title.');
    } finally {
      setSaving(false);
    }
  }

  if (editing) {
    return (
      <div className="flex w-full min-w-0 flex-col gap-1">
        <div className="flex min-w-0 items-center gap-2">
          <TextInput
            aria-label="Title"
            autoFocus
            sizing="lg"
            theme={{ field: { input: { sizes: { lg: 'p-2 text-2xl' } } } }}
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault();
                void save();
              } else if (event.key === 'Escape') {
                event.preventDefault();
                cancelEditing();
              }
            }}
            className="min-w-0 flex-1"
          />
          <Button
            size="sm"
            color="gray"
            outline
            aria-label="Save title"
            title="Save title"
            disabled={saving}
            onClick={() => void save()}
          >
            <HiCheck aria-hidden="true" className="h-5 w-5" />
          </Button>
          <Button
            size="sm"
            color="gray"
            outline
            aria-label="Cancel title edit"
            title="Cancel title edit"
            disabled={saving}
            onClick={cancelEditing}
          >
            <HiXMark aria-hidden="true" className="h-5 w-5" />
          </Button>
        </div>
        {error === null ? null : (
          <p className="text-danger text-sm" role="alert">
            {error}
          </p>
        )}
      </div>
    );
  }

  return (
    <div className="flex min-w-0 items-center gap-2">
      <h1 className="min-w-0 text-2xl font-semibold wrap-break-word">{title}</h1>
      <button
        type="button"
        aria-label="Edit title"
        title="Edit title"
        onClick={startEditing}
        className={ICON_BUTTON_CLASS}
      >
        <HiPencil aria-hidden="true" className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}

export type TaskHeaderProps = {
  project: ProjectSnapshot;
  task: Task;
  handle: string;
  onSaveTitle: (title: string) => Promise<void>;
  onBack: () => void;
};

export function TaskHeader(props: TaskHeaderProps) {
  const { project, task, handle, onSaveTitle, onBack } = props;

  const location = useLocation();

  return (
    <div className="flex flex-col gap-3">
      <Link
        to={`/projects/${project.id}/tasks${location.search}`}
        aria-label="Back to board"
        title="Back to board"
        className="border-line text-muted hover:border-line-hover inline-flex h-11 w-11 items-center justify-center rounded-lg border transition-colors hover:text-zinc-200 md:h-8 md:w-8"
        onClick={(event) => {
          event.preventDefault();
          onBack();
        }}
      >
        <svg
          aria-hidden="true"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth={1.8}
          className="h-5 w-5 md:h-4 md:w-4"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18"
          />
        </svg>
      </Link>
      <div className="flex min-w-0 flex-col gap-1">
        <TaskTitle title={task.title} onSave={onSaveTitle} />
        <div className="flex min-w-0 flex-wrap items-baseline justify-between gap-2">
          <p className="text-muted text-sm">{project.name}</p>
          <span className="text-muted shrink-0 font-mono text-sm">{handle}</span>
        </div>
      </div>
    </div>
  );
}
