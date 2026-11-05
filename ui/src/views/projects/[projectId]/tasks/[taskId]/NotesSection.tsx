import { Button } from 'flowbite-react';
import { useState } from 'react';
import { TextAreaControl } from '@/components/form-controls/TextAreaControl';
import type { Note } from '@/generated/Note';

export type NotesSectionProps = {
  notes: Note[];
  onCreate: (contents: string) => Promise<void>;
  onUpdate: (note: Note, contents: string) => Promise<void>;
  onDelete: (note: Note) => Promise<void>;
};

export function NotesSection(props: NotesSectionProps) {
  const { notes, onCreate, onUpdate, onDelete } = props;

  const [draft, setDraft] = useState('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editContents, setEditContents] = useState('');
  const [confirmingId, setConfirmingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const ordered = [...notes].sort((left, right) => left.created_at.localeCompare(right.created_at));

  async function handleCreate(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (draft.trim().length === 0) {
      return;
    }

    setError(null);
    try {
      await onCreate(draft);
      setDraft('');
    } catch {
      setError('Could not save this note.');
    }
  }

  function startEditing(note: Note) {
    setEditingId(note.id);
    setEditContents(note.contents);
    setConfirmingId(null);
  }

  async function handleUpdate(event: React.FormEvent<HTMLFormElement>, note: Note) {
    event.preventDefault();
    if (editContents.trim().length === 0) {
      setError('A note cannot be empty.');
      return;
    }

    setError(null);
    try {
      await onUpdate(note, editContents);
      setEditingId(null);
    } catch {
      setError('Could not save this note.');
    }
  }

  async function handleDelete(note: Note) {
    setError(null);
    try {
      await onDelete(note);
      setConfirmingId(null);
    } catch {
      setError('Could not delete this note.');
    }
  }

  return (
    <section className="flex flex-col gap-3">
      <h2 className="text-lg font-semibold">Notes</h2>
      {ordered.length === 0 ? <p className="text-faint text-sm">No notes yet.</p> : null}
      {ordered.map((note) => (
        <div
          key={note.id}
          className="border-line-strong bg-surface flex flex-col gap-2 rounded-lg border p-3"
        >
          {editingId === note.id ? (
            <form
              onSubmit={(event) => void handleUpdate(event, note)}
              className="flex flex-col gap-2"
            >
              <TextAreaControl
                label="Note contents"
                value={editContents}
                onChange={setEditContents}
                rows={3}
              />
              <div className="flex gap-2">
                <Button type="submit" size="xs">
                  Save note
                </Button>
                <Button
                  size="xs"
                  color="alternative"
                  type="button"
                  onClick={() => setEditingId(null)}
                >
                  Cancel
                </Button>
              </div>
            </form>
          ) : (
            <>
              <p className="text-sm whitespace-pre-wrap">{note.contents}</p>
              <p className="text-faint text-xs">{new Date(note.created_at).toLocaleString()}</p>
              {confirmingId === note.id ? (
                <div className="flex items-center gap-2">
                  <span className="text-muted text-sm">Delete this note?</span>
                  <Button
                    size="xs"
                    color="red"
                    aria-label="Confirm delete note"
                    onClick={() => void handleDelete(note)}
                  >
                    Confirm
                  </Button>
                  <Button size="xs" color="alternative" onClick={() => setConfirmingId(null)}>
                    Cancel
                  </Button>
                </div>
              ) : (
                <div className="flex gap-2">
                  <Button
                    size="xs"
                    color="alternative"
                    aria-label="Edit note"
                    onClick={() => startEditing(note)}
                  >
                    Edit
                  </Button>
                  <Button
                    size="xs"
                    color="red"
                    aria-label="Delete note"
                    onClick={() => setConfirmingId(note.id)}
                  >
                    Delete
                  </Button>
                </div>
              )}
            </>
          )}
        </div>
      ))}
      <form onSubmit={(event) => void handleCreate(event)} className="flex flex-col gap-2">
        <TextAreaControl
          label="New note"
          value={draft}
          onChange={setDraft}
          placeholder="Add a note"
          rows={2}
        />
        <div>
          <Button type="submit" size="xs" disabled={draft.trim().length === 0}>
            Add note
          </Button>
        </div>
      </form>
      {error === null ? null : (
        <p className="text-danger text-sm" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
