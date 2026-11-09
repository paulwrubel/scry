import { Button } from 'flowbite-react';
import { useState } from 'react';
import { z } from 'zod';
import type { Note } from '@/generated/Note';
import { useAppForm } from '@/hooks/useAppForm';

const schema = z.object({
  contents: z.string().trim().min(1, 'A note cannot be empty.'),
});

export type NotesSectionProps = {
  notes: Note[];
  onCreate: (contents: string) => Promise<void>;
  onUpdate: (note: Note, contents: string) => Promise<void>;
  onDelete: (note: Note) => Promise<void>;
};

export function NotesSection(props: NotesSectionProps) {
  const { notes, onCreate, onUpdate, onDelete } = props;

  const [editingId, setEditingId] = useState<string | null>(null);
  const [confirmingId, setConfirmingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const ordered = [...notes].sort((left, right) => left.created_at.localeCompare(right.created_at));

  const createForm = useAppForm({
    defaultValues: { contents: '' },
    validators: { onChange: schema, onMount: schema },
    listeners: { onChange: () => setError(null) },
    onSubmit: async ({ value }) => {
      setError(null);
      try {
        await onCreate(value.contents.trim());
        createForm.reset();
        void createForm.validate('change');
      } catch {
        setError('Could not save this note.');
      }
    },
  });

  const editForm = useAppForm({
    defaultValues: { contents: '' },
    validators: { onChange: schema, onMount: schema },
    listeners: { onChange: () => setError(null) },
    onSubmit: async ({ value }) => {
      const note = notes.find((item) => item.id === editingId);
      if (note === undefined) {
        return;
      }

      setError(null);
      try {
        await onUpdate(note, value.contents.trim());
        setEditingId(null);
        editForm.reset();
      } catch {
        setError('Could not save this note.');
      }
    },
  });

  function startEditing(note: Note) {
    editForm.reset();
    editForm.setFieldValue('contents', note.contents);
    setEditingId(note.id);
    setConfirmingId(null);
  }

  function cancelEditing() {
    setEditingId(null);
    editForm.reset();
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
              onSubmit={(event) => {
                event.preventDefault();
                void editForm.handleSubmit();
              }}
              className="flex flex-col gap-2"
            >
              <editForm.AppField name="contents">
                {(field) => <field.FormTextAreaField label="Note contents" rows={3} />}
              </editForm.AppField>
              <div className="flex gap-2">
                <editForm.Subscribe
                  selector={(state) => ({
                    isValid: state.isValid,
                    submitting: state.isSubmitting,
                  })}
                >
                  {({ isValid, submitting }) => (
                    <Button type="submit" size="xs" disabled={submitting || !isValid}>
                      Save note
                    </Button>
                  )}
                </editForm.Subscribe>
                <Button size="xs" color="alternative" type="button" onClick={cancelEditing}>
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
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void createForm.handleSubmit();
        }}
        className="flex flex-col gap-2"
      >
        <createForm.AppField name="contents">
          {(field) => (
            <field.FormTextAreaField label="New note" placeholder="Add a note" rows={2} />
          )}
        </createForm.AppField>
        <div>
          <createForm.Subscribe
            selector={(state) => ({
              isValid: state.isValid,
              submitting: state.isSubmitting,
            })}
          >
            {({ isValid, submitting }) => (
              <Button type="submit" size="xs" disabled={submitting || !isValid}>
                Add note
              </Button>
            )}
          </createForm.Subscribe>
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
