import { Badge, Button, TextInput } from 'flowbite-react';
import { useState } from 'react';
import { HiPencil, HiTrash } from 'react-icons/hi2';
import { SiGithub } from 'react-icons/si';
import { parseGithubIdentity } from '@/contract/github';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { Task } from '@/generated/Task';
import {
  externalError,
  githubIdentity,
  githubLabel,
  githubUrl,
  lastSyncedAt,
} from '@/utils/external';

const ICON_BUTTON_CLASS =
  'text-muted inline-flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center border-0 bg-transparent p-0 transition-colors hover:text-zinc-200 focus:outline-none focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50';

type ExternalLinkFieldProps = {
  task: Task;
  onSave: (identity: ExternalIdentity | null) => Promise<void>;
};

export function ExternalLinkField(props: ExternalLinkFieldProps) {
  const { task, onSave } = props;

  const identity = githubIdentity(task);
  const url = githubUrl(task);
  const label = githubLabel(task);
  const syncError = externalError(task);
  const syncedAt = lastSyncedAt(task);

  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(url ?? '');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function startEditing() {
    setDraft(url ?? '');
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
      setSaving(true);
      setError(null);
      try {
        await onSave(null);
        setEditing(false);
      } catch {
        setError('Could not save the GitHub link.');
      } finally {
        setSaving(false);
      }
      return;
    }

    const nextIdentity = parseGithubIdentity(trimmed);
    if (nextIdentity === null) {
      setError('Enter a valid GitHub issue or pull request URL.');
      return;
    }

    setSaving(true);
    setError(null);
    try {
      await onSave(nextIdentity);
      setEditing(false);
    } catch {
      setError('Could not save the GitHub link.');
    } finally {
      setSaving(false);
    }
  }

  async function remove() {
    setSaving(true);
    setError(null);
    try {
      await onSave(null);
    } catch {
      setError('Could not save the GitHub link.');
    } finally {
      setSaving(false);
    }
  }

  if (editing) {
    return (
      <div className="flex flex-col gap-1">
        <TextInput
          aria-label="GitHub URL"
          autoFocus
          value={draft}
          placeholder="https://github.com/owner/repo/issues/1"
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
        />
        <div className="flex items-center gap-2">
          <Button
            size="xs"
            color="alternative"
            aria-label="Cancel GitHub link edit"
            disabled={saving}
            onClick={cancelEditing}
          >
            Cancel
          </Button>
          <Button
            size="xs"
            aria-label="Save GitHub link"
            disabled={saving}
            onClick={() => void save()}
          >
            {saving ? 'Saving...' : 'Save'}
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
    <div className="flex flex-col gap-1">
      {identity === null || url === null || label === null ? (
        <div className="flex items-center gap-2">
          <p className="text-faint text-sm">No GitHub link.</p>
          <Button size="xs" color="alternative" aria-label="Link GitHub" onClick={startEditing}>
            Link GitHub
          </Button>
        </div>
      ) : (
        <>
          <div className="flex min-w-0 items-center gap-2">
            <SiGithub aria-hidden="true" className="text-muted h-4 w-4 shrink-0" />
            <a
              href={url}
              target="_blank"
              rel="noreferrer"
              className="min-w-0 truncate text-sm text-blue-400 hover:underline"
            >
              {label}
            </a>
            <button
              type="button"
              aria-label="Edit GitHub link"
              title="Edit GitHub link"
              onClick={startEditing}
              className={ICON_BUTTON_CLASS}
            >
              <HiPencil aria-hidden="true" className="h-3.5 w-3.5" />
            </button>
            <button
              type="button"
              aria-label="Remove GitHub link"
              title="Remove GitHub link"
              disabled={saving}
              onClick={() => void remove()}
              className={ICON_BUTTON_CLASS}
            >
              <HiTrash aria-hidden="true" className="h-3.5 w-3.5" />
            </button>
          </div>
          <p className="text-faint text-xs">
            {syncedAt === null
              ? 'Not synced yet.'
              : `Last synced ${new Date(syncedAt).toLocaleString()}`}
          </p>
          {syncError === null ? null : <Badge color="failure">GitHub: {syncError}</Badge>}
        </>
      )}
      {error === null ? null : (
        <p className="text-danger text-sm" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
