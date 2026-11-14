import { useCallback, useEffect, useRef, useState } from 'react';
import { z } from 'zod';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import { useAppForm } from '@/hooks/useAppForm';
import { parseTaskInput } from '@/utils/tasks/input';

const schema = z.object({
  task: z
    .string()
    .trim()
    .min(1, 'Enter a task title or GitHub URL.')
    .refine(
      (value) => parseTaskInput(value) !== null,
      'Only GitHub issue or pull request URLs are supported.',
    ),
});

export type CreateTaskHandler = (
  title: string,
  externalIdentity: ExternalIdentity | null,
) => Promise<void>;

export function useCreateTaskForm(onCreate: CreateTaskHandler) {
  const [open, setOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // the form keeps its first submit handler, so always call the latest callback
  const onCreateRef = useRef(onCreate);
  useEffect(() => {
    onCreateRef.current = onCreate;
  }, [onCreate]);

  const form = useAppForm({
    defaultValues: { task: '' },
    validators: { onChange: schema, onMount: schema },
    listeners: { onChange: () => setError(null) },
    onSubmit: async ({ value }) => {
      const submitted = value.task;
      const parsed = parseTaskInput(value.task);
      if (parsed === null) {
        return;
      }
      setError(null);
      try {
        await onCreateRef.current(parsed.title, parsed.externalIdentity);
        if (form.getFieldValue('task') === submitted) {
          form.reset();
          void form.validate('change');
        }
        inputRef.current?.focus();
      } catch {
        setError('Could not add this task.');
      }
    },
  });

  const closeForm = useCallback(() => {
    setOpen(false);
    form.reset();
    void form.validate('change');
    setError(null);
  }, [form]);

  const openForm = useCallback(() => setOpen(true), []);

  return { form, open, error, inputRef, openForm, closeForm };
}

export type UseCreateTaskFormResult = ReturnType<typeof useCreateTaskForm>;
