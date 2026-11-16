import { Button, Popover } from 'flowbite-react';
import type { PopoverTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { useState } from 'react';
import type { SetStateAction } from 'react';
import { dueFromInputs, dueToInputs, formatDue } from '@/contract/due';
import type { Task } from '@/generated/Task';
import type { When } from '@/generated/When';
import { useAppForm } from '@/hooks/useAppForm';

const PILL_CLASS =
  'border-line bg-panel text-zinc-200 cursor-pointer rounded-full border px-3 py-1 text-xs disabled:cursor-not-allowed disabled:opacity-60';

const INPUT_CLASS =
  'border-line-strong bg-surface text-zinc-200 focus:border-line-hover rounded-lg border p-2 text-sm focus:outline-none';

const popoverTheme: DeepPartial<PopoverTheme> = {
  base: 'absolute z-30 rounded-lg border border-line bg-panel shadow-lg outline-none',
  content: 'rounded-lg',
};

export type DueBadgeProps = {
  task: Task;
  onSave: (due: When | null) => Promise<void>;
};

export function DueBadge(props: DueBadgeProps) {
  const { task, onSave } = props;

  const [open, setOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function save(due: When | null) {
    setSaving(true);
    setError(null);
    try {
      await onSave(due);
      form.reset();
      setOpen(false);
    } catch {
      setError('Could not save the due date.');
    } finally {
      setSaving(false);
    }
  }

  const form = useAppForm({
    defaultValues: { date: '', time: '' },
    onSubmit: async ({ value }) => {
      await save(dueFromInputs(value.date, value.time));
    },
  });

  function handleOpenChange(next: SetStateAction<boolean>) {
    const nextOpen = typeof next === 'function' ? next(open) : next;
    if (nextOpen && !open) {
      const inputs = dueToInputs(task.due);
      form.setFieldValue('date', inputs.date);
      form.setFieldValue('time', inputs.time);
      setError(null);
    }
    setOpen(nextOpen);
  }

  function cancelEditing() {
    form.reset();
    setError(null);
    setOpen(false);
  }

  const label = task.due === null ? null : formatDue(task.due);

  return (
    <Popover
      arrow={false}
      placement="bottom-start"
      trigger="click"
      open={open}
      onOpenChange={handleOpenChange}
      theme={popoverTheme}
      content={
        <form
          className="flex flex-col gap-2 p-3"
          onSubmit={(event) => {
            event.preventDefault();
            void form.handleSubmit();
          }}
        >
          <div className="flex items-center gap-2">
            <form.AppField name="date">
              {(field) => (
                <input
                  type="date"
                  aria-label="Due date"
                  required
                  value={field.state.value}
                  disabled={saving}
                  onChange={(event) => field.handleChange(event.target.value)}
                  className={INPUT_CLASS}
                />
              )}
            </form.AppField>
            <form.AppField name="time">
              {(field) => (
                <input
                  type="time"
                  aria-label="Due time"
                  value={field.state.value}
                  disabled={saving}
                  onChange={(event) => field.handleChange(event.target.value)}
                  className={INPUT_CLASS}
                />
              )}
            </form.AppField>
          </div>
          <div className="flex items-center justify-between gap-2">
            <Button
              type="button"
              size="xs"
              color="alternative"
              disabled={saving}
              onClick={cancelEditing}
            >
              Cancel
            </Button>
            <div className="flex items-center gap-2">
              <Button
                type="button"
                size="xs"
                color="light"
                disabled={saving || task.due === null}
                onClick={() => void save(null)}
              >
                Clear
              </Button>
              <Button type="submit" size="xs" disabled={saving}>
                {saving ? 'Saving...' : 'Save'}
              </Button>
            </div>
          </div>
          {error === null ? null : (
            <p className="text-danger text-xs" role="alert">
              {error}
            </p>
          )}
        </form>
      }
    >
      <button
        type="button"
        aria-label={label === null ? 'No due date' : `Due ${label}`}
        title={label === null ? 'No due date' : `Due ${label}`}
        disabled={saving}
        className={PILL_CLASS}
      >
        {label ?? 'No due date'}
      </button>
    </Popover>
  );
}
