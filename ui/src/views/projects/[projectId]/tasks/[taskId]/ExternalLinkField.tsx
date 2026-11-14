import { Badge, Button } from 'flowbite-react';
import { useState } from 'react';
import { HiPencil, HiTrash } from 'react-icons/hi2';
import { SiGithub } from 'react-icons/si';
import { VscGitPullRequest, VscIssues } from 'react-icons/vsc';
import { z } from 'zod';
import { parseGithubIdentity } from '@/contract/github';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { Task } from '@/generated/Task';
import { useAppForm } from '@/hooks/useAppForm';
import { formatDateTime } from '@/utils/date';
import {
  externalError,
  githubIconColor,
  githubIdentity,
  githubKind,
  githubLabel,
  githubUrl,
  lastSyncedAt,
} from '@/utils/github';

const ICON_BUTTON_CLASS =
  'text-muted inline-flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center border-0 bg-transparent p-0 transition-colors hover:text-zinc-200 focus:outline-none focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50';

const schema = z.object({
  url: z
    .string()
    .trim()
    .refine(
      (value) => value === '' || parseGithubIdentity(value) !== null,
      'Enter a valid GitHub issue or pull request URL.',
    ),
});

export type ExternalLinkFieldProps = {
  task: Task;
  onSave: (identity: ExternalIdentity | null) => Promise<void>;
};

export function ExternalLinkField(props: ExternalLinkFieldProps) {
  const { task, onSave } = props;

  const identity = githubIdentity(task);
  const url = githubUrl(task);
  const label = githubLabel(task);
  const kind = githubKind(task);
  const KindIcon = kind === 'pull' ? VscGitPullRequest : kind === 'issue' ? VscIssues : SiGithub;
  const kindLabel = kind === 'pull' ? 'Pull request' : kind === 'issue' ? 'Issue' : 'GitHub';
  const syncError = externalError(task);
  const syncedAt = lastSyncedAt(task);

  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const form = useAppForm({
    defaultValues: { url: url ?? '' },
    validators: { onChange: schema, onMount: schema },
    onSubmit: async ({ value }) => {
      const trimmed = value.url.trim();
      setSaving(true);
      setError(null);
      try {
        if (trimmed === '') {
          await onSave(null);
        } else {
          const nextIdentity = parseGithubIdentity(trimmed);
          if (nextIdentity === null) {
            return;
          }
          await onSave(nextIdentity);
        }
        setEditing(false);
      } catch {
        setError('Could not save the GitHub link.');
      } finally {
        setSaving(false);
      }
    },
  });

  function startEditing() {
    form.reset();
    form.setFieldValue('url', url ?? '');
    setError(null);
    setEditing(true);
  }

  function cancelEditing() {
    form.reset();
    setError(null);
    setEditing(false);
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
        <form
          className="flex flex-col gap-1"
          onSubmit={(event) => {
            event.preventDefault();
            void form.handleSubmit();
          }}
          onKeyDown={(event) => {
            if (event.key === 'Escape') {
              event.preventDefault();
              cancelEditing();
            }
          }}
        >
          <form.AppField name="url">
            {(field) => (
              <field.FormTextField
                label="GitHub URL"
                hideLabel
                autoFocus
                placeholder="https://github.com/owner/repo/issues/1"
              />
            )}
          </form.AppField>
          <form.Subscribe
            selector={(state) => ({ isValid: state.isValid, isSubmitting: state.isSubmitting })}
          >
            {({ isValid, isSubmitting }) => (
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
                  type="submit"
                  size="xs"
                  aria-label="Save GitHub link"
                  disabled={!isValid || isSubmitting}
                >
                  {saving ? 'Saving...' : 'Save'}
                </Button>
              </div>
            )}
          </form.Subscribe>
        </form>
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
            <KindIcon aria-hidden="true" className={`${githubIconColor(task)} h-4 w-4 shrink-0`} />
            <a
              href={url}
              target="_blank"
              rel="noreferrer"
              className="min-w-0 truncate text-sm text-blue-400 hover:underline"
            >
              {label}
            </a>
            <Badge color="gray">{kindLabel}</Badge>
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
            {syncedAt === null ? 'Not synced yet.' : `Last synced ${formatDateTime(syncedAt)}`}
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
