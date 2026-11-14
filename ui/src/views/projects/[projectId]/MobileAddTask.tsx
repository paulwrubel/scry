import { Button, Drawer } from 'flowbite-react';
import type { DrawerTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { useEffect } from 'react';
import { HiPlus } from 'react-icons/hi2';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import { useCloseRequest } from '@/hooks/useCloseRequest';
import { useCreateTaskForm } from '@/hooks/useCreateTaskForm';
import { useVisualViewportInset } from '@/hooks/useVisualViewportInset';

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
  onCreate: (title: string, externalIdentity: ExternalIdentity | null) => Promise<void>;
};

export function MobileAddTask(props: MobileAddTaskProps) {
  const { onCreate } = props;

  const { form, open, error, inputRef, openForm, closeForm } = useCreateTaskForm(onCreate);
  const bottomInset = useVisualViewportInset();

  useEffect(() => {
    if (open) {
      inputRef.current?.focus();
    }
  }, [open, inputRef]);

  useCloseRequest(open, closeForm);

  return (
    <>
      <button
        type="button"
        aria-label="Add task"
        onClick={openForm}
        className="bg-primary-600 hover:bg-primary-700 focus-visible:ring-primary-300 fixed right-4 bottom-[calc(env(safe-area-inset-bottom)+4rem)] z-20 flex h-14 w-14 items-center justify-center rounded-full text-white shadow-lg transition-colors focus:outline-none focus-visible:ring-2 md:hidden"
      >
        <HiPlus aria-hidden="true" className="h-6 w-6" />
      </button>
      <Drawer
        open={open}
        onClose={closeForm}
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
              <form.AppField name="task">
                {(field) => (
                  <field.FormTextField
                    label="Task title or GitHub URL"
                    hideLabel
                    placeholder="Task title or GitHub URL"
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
