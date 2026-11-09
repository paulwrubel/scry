import { Button, Drawer } from 'flowbite-react';
import type { DrawerTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { useEffect, useRef, useState } from 'react';
import { HiPlus } from 'react-icons/hi2';
import { z } from 'zod';
import { useAppForm } from '@/hooks/useAppForm';
import { useVisualViewportInset } from '@/hooks/useVisualViewportInset';

const schema = z.object({
  title: z.string().trim().min(1, 'A task needs a title.'),
});

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
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const bottomInset = useVisualViewportInset();

  const form = useAppForm({
    defaultValues: { title: '' },
    validators: { onChange: schema, onMount: schema },
    listeners: { onChange: () => setError(null) },
    onSubmit: async ({ value }) => {
      const submitted = value.title;
      setError(null);
      try {
        await onCreate(submitted.trim());
        if (form.getFieldValue('title') === submitted) {
          form.reset();
          void form.validate('change');
        }
        inputRef.current?.focus();
      } catch {
        setError('Could not add this task.');
      }
    },
  });

  useEffect(() => {
    if (open) {
      inputRef.current?.focus();
    }
  }, [open]);

  function close() {
    setOpen(false);
    form.reset();
    void form.validate('change');
    setError(null);
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
        style={{ bottom: bottomInset }}
      >
        <div className="flex flex-col gap-3 p-4 pb-[calc(env(safe-area-inset-bottom)+1rem)]">
          <div aria-hidden="true" className="bg-line-strong mx-auto h-1 w-10 rounded-full" />
          <form
            onSubmit={(event) => {
              event.preventDefault();
              void form.handleSubmit();
            }}
            className="flex items-center gap-2"
          >
            <div className="min-w-0 flex-1">
              <form.AppField name="title">
                {(field) => (
                  <field.FormTextField
                    label="Task title"
                    hideLabel
                    placeholder="Task title"
                    inputRef={inputRef}
                  />
                )}
              </form.AppField>
            </div>
            <form.Subscribe
              selector={(state) => ({
                isValid: state.isValid,
                submitting: state.isSubmitting,
              })}
            >
              {({ isValid, submitting }) => (
                <Button type="submit" disabled={submitting || !isValid}>
                  Add
                </Button>
              )}
            </form.Subscribe>
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
