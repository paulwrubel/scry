import { Button } from 'flowbite-react';
import { useState } from 'react';
import { z } from 'zod';
import type { Comment } from '@/generated/Comment';
import { useAppForm } from '@/hooks/useAppForm';

const schema = z.object({
  contents: z.string().trim().min(1, 'A comment cannot be empty.'),
});

export type CommentsSectionProps = {
  comments: Comment[];
  onCreate: (contents: string) => Promise<void>;
  onUpdate: (comment: Comment, contents: string) => Promise<void>;
  onDelete: (comment: Comment) => Promise<void>;
};

export function CommentsSection(props: CommentsSectionProps) {
  const { comments, onCreate, onUpdate, onDelete } = props;

  const [editingId, setEditingId] = useState<string | null>(null);
  const [confirmingId, setConfirmingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const ordered = [...comments].sort((left, right) =>
    left.created_at.localeCompare(right.created_at),
  );

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
        setError('Could not save this comment.');
      }
    },
  });

  const editForm = useAppForm({
    defaultValues: { contents: '' },
    validators: { onChange: schema, onMount: schema },
    listeners: { onChange: () => setError(null) },
    onSubmit: async ({ value }) => {
      const comment = comments.find((item) => item.id === editingId);
      if (comment === undefined) {
        return;
      }

      setError(null);
      try {
        await onUpdate(comment, value.contents.trim());
        setEditingId(null);
        editForm.reset();
      } catch {
        setError('Could not save this comment.');
      }
    },
  });

  function startEditing(comment: Comment) {
    editForm.reset();
    editForm.setFieldValue('contents', comment.contents);
    setEditingId(comment.id);
    setConfirmingId(null);
  }

  function cancelEditing() {
    setEditingId(null);
    editForm.reset();
  }

  async function handleDelete(comment: Comment) {
    setError(null);
    try {
      await onDelete(comment);
      setConfirmingId(null);
    } catch {
      setError('Could not delete this comment.');
    }
  }

  return (
    <section className="flex flex-col gap-3">
      <h2 className="text-lg font-semibold">Comments</h2>
      {ordered.length === 0 ? <p className="text-faint text-sm">No comments yet.</p> : null}
      {ordered.map((comment) => (
        <div
          key={comment.id}
          className="border-line-strong bg-surface flex flex-col gap-2 rounded-lg border p-3"
        >
          {editingId === comment.id ? (
            <form
              onSubmit={(event) => {
                event.preventDefault();
                void editForm.handleSubmit();
              }}
              className="flex flex-col gap-2"
            >
              <editForm.AppField name="contents">
                {(field) => <field.FormTextAreaField label="Comment contents" rows={3} />}
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
                      Save comment
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
              <p className="text-sm whitespace-pre-wrap">{comment.contents}</p>
              <p className="text-faint text-xs">{new Date(comment.created_at).toLocaleString()}</p>
              {confirmingId === comment.id ? (
                <div className="flex items-center gap-2">
                  <span className="text-muted text-sm">Delete this comment?</span>
                  <Button
                    size="xs"
                    color="red"
                    aria-label="Confirm delete comment"
                    onClick={() => void handleDelete(comment)}
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
                    aria-label="Edit comment"
                    onClick={() => startEditing(comment)}
                  >
                    Edit
                  </Button>
                  <Button
                    size="xs"
                    color="red"
                    aria-label="Delete comment"
                    onClick={() => setConfirmingId(comment.id)}
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
            <field.FormTextAreaField label="New comment" placeholder="Add a comment" rows={2} />
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
                Add comment
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
