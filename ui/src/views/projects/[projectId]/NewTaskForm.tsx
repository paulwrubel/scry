import { Button, TextInput } from 'flowbite-react';
import { useEffect, useRef, useState } from 'react';

type NewTaskFormProps = {
  onCreate: (title: string) => Promise<void>;
};

export function NewTaskForm(props: NewTaskFormProps) {
  const { onCreate } = props;

  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) {
      inputRef.current?.focus();
    }
  }, [open]);

  if (!open) {
    return (
      <Button size="xs" color="alternative" className="shrink-0" onClick={() => setOpen(true)}>
        Add task
      </Button>
    );
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) {
      return;
    }

    const trimmed = title.trim();
    if (trimmed.length === 0) {
      return;
    }

    const submitted = title;
    setSubmitting(true);
    try {
      await onCreate(trimmed);
      setTitle((current) => (current === submitted ? '' : current));
      inputRef.current?.focus();
    } finally {
      setSubmitting(false);
    }
  }

  function handleCancel() {
    setTitle('');
    setOpen(false);
  }

  function handleKeyDown(event: React.KeyboardEvent<HTMLFormElement>) {
    if (event.key === 'Escape') {
      event.preventDefault();
      handleCancel();
    }
  }

  return (
    <form
      onSubmit={handleSubmit}
      onKeyDown={handleKeyDown}
      className="flex shrink-0 flex-col gap-2"
    >
      <TextInput
        ref={inputRef}
        sizing="sm"
        value={title}
        onChange={(event) => setTitle(event.target.value)}
        placeholder="Task title"
        aria-label="Task title"
      />
      <div className="flex gap-2">
        <Button type="submit" size="xs" disabled={submitting || title.trim().length === 0}>
          Add
        </Button>
        <Button type="button" size="xs" color="alternative" onClick={handleCancel}>
          Cancel
        </Button>
      </div>
    </form>
  );
}
