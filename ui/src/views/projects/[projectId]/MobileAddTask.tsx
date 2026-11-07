import { Button, Drawer, TextInput } from 'flowbite-react';
import type { DrawerTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { useEffect, useRef, useState } from 'react';
import { HiPlus } from 'react-icons/hi2';

const sheetTheme: DeepPartial<DrawerTheme> = {
  root: {
    base: 'fixed z-40 overflow-y-auto border-line bg-surface p-0 transition-transform duration-300 ease-out',
    position: {
      bottom: {
        on: 'inset-x-0 bottom-0 w-full rounded-t-xl border-t pb-[env(safe-area-inset-bottom)]',
        off: 'inset-x-0 bottom-0 w-full rounded-t-xl border-t pb-[env(safe-area-inset-bottom)] translate-y-full',
      },
    },
  },
};

export type MobileAddTaskProps = {
  onCreate: (title: string) => Promise<void>;
};

export function MobileAddTask(props: MobileAddTaskProps) {
  const { onCreate } = props;

  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) {
      inputRef.current?.focus();
    }
  }, [open]);

  function close() {
    setOpen(false);
    setTitle('');
    setError(null);
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
    setError(null);
    try {
      await onCreate(trimmed);
      setTitle((current) => (current === submitted ? '' : current));
      inputRef.current?.focus();
    } catch {
      setError('Could not add this task.');
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <button
        type="button"
        aria-label="Add task"
        onClick={() => setOpen(true)}
        className="bg-primary-600 hover:bg-primary-700 focus-visible:ring-primary-300 fixed right-4 bottom-[calc(env(safe-area-inset-bottom)+4rem)] z-20 flex h-14 w-14 items-center justify-center rounded-full text-white shadow-lg transition-colors focus:outline-none focus-visible:ring-2 md:hidden"
      >
        <HiPlus aria-hidden="true" className="h-6 w-6" />
      </button>
      <Drawer
        open={open}
        onClose={close}
        position="bottom"
        theme={sheetTheme}
        aria-hidden={!open}
        inert={!open}
      >
        <div className="flex flex-col gap-3 p-4 pb-[calc(env(safe-area-inset-bottom)+1rem)]">
          <div aria-hidden="true" className="bg-line-strong mx-auto h-1 w-10 rounded-full" />
          <form onSubmit={handleSubmit} className="flex items-center gap-2">
            <div className="min-w-0 flex-1">
              <TextInput
                ref={inputRef}
                value={title}
                onChange={(event) => {
                  setTitle(event.target.value);
                  setError(null);
                }}
                placeholder="Task title"
                aria-label="Task title"
              />
            </div>
            <Button type="submit" disabled={submitting || title.trim().length === 0}>
              Add
            </Button>
          </form>
          {error === null ? null : (
            <p className="text-danger text-sm" role="alert">
              {error}
            </p>
          )}
        </div>
      </Drawer>
    </>
  );
}
