import { useState } from 'react';
import { HiChevronDown } from 'react-icons/hi2';
import type { Priority } from '@/generated/Priority';
import type { Task } from '@/generated/Task';
import {
  CHEVRON_CLASS,
  PRIORITIES,
  PRIORITY_DOT_CLASS,
  PRIORITY_LABELS,
  SELECT_CLASS,
} from './constants';

export type PriorityBadgeProps = {
  task: Task;
  onSave: (priority: Priority) => Promise<void>;
};

export function PriorityBadge(props: PriorityBadgeProps) {
  const { task, onSave } = props;

  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function change(priority: Priority) {
    if (priority === task.priority) {
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await onSave(priority);
    } catch {
      setError('Could not save the priority.');
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="flex flex-col gap-1">
      <div className="relative inline-flex">
        <select
          aria-label="Priority"
          value={task.priority}
          disabled={saving}
          // the select options come from PRIORITIES, so the value is always a Priority
          onChange={(event) => void change(event.target.value as Priority)}
          className={SELECT_CLASS}
        >
          {PRIORITIES.map((priority) => (
            <option key={priority} value={priority}>
              {PRIORITY_LABELS[priority]}
            </option>
          ))}
        </select>
        {task.priority === 'critical' || task.priority === 'elevated' ? (
          <span
            aria-hidden="true"
            className={`pointer-events-none absolute top-1/2 left-2.5 h-1.5 w-1.5 -translate-y-1/2 rounded-full ${PRIORITY_DOT_CLASS[task.priority]}`}
          />
        ) : null}
        <HiChevronDown aria-hidden="true" className={CHEVRON_CLASS} />
      </div>
      {error === null ? null : (
        <p className="text-danger text-xs" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
