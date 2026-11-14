import { useState } from 'react';
import type { Priority } from '@/generated/Priority';
import type { Task } from '@/generated/Task';
import { PRIORITIES, PRIORITY_DOT_CLASS, PRIORITY_LABELS } from '@/data/priority';
import { MetaSelect } from './MetaSelect';

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
    <MetaSelect
      ariaLabel="Priority"
      value={task.priority}
      disabled={saving}
      // the select options come from PRIORITIES, so the value is always a Priority
      onChange={(value) => void change(value as Priority)}
      dot={
        task.priority === 'critical' || task.priority === 'elevated' ? (
          <span
            aria-hidden="true"
            className={`pointer-events-none absolute top-1/2 left-2.5 h-1.5 w-1.5 -translate-y-1/2 rounded-full ${PRIORITY_DOT_CLASS[task.priority]}`}
          />
        ) : null
      }
      error={error}
    >
      {PRIORITIES.map((priority) => (
        <option key={priority} value={priority}>
          {PRIORITY_LABELS[priority]}
        </option>
      ))}
    </MetaSelect>
  );
}
