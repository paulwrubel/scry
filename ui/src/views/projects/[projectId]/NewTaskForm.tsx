import { Button, TextInput } from 'flowbite-react';
import { useState } from 'react';

type NewTaskFormProps = {
  onCreate: (title: string) => Promise<void>;
};

export function NewTaskForm(props: NewTaskFormProps) {
  const { onCreate } = props;

  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState('');
  const [submitting, setSubmitting] = useState(false);

  if (!open) {
    return (
      <Button size="xs" color="alternative" onClick={() => setOpen(true)}>
        Add task
      </Button>
    );
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = title.trim();
    if (trimmed.length === 0) {
      return;
    }

    setSubmitting(true);
    try {
      await onCreate(trimmed);
      setTitle('');
      setOpen(false);
    } finally {
      setSubmitting(false);
    }
  }

  function handleCancel() {
    setTitle('');
    setOpen(false);
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-2">
      <TextInput
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
