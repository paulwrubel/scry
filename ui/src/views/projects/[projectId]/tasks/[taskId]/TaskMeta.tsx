import { useState } from 'react';
import { HiChevronDown } from 'react-icons/hi2';
import type { Priority } from '@/generated/Priority';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { SECTION_COLOR_HEX } from '@/utils/sectionColor';

const PRIORITY_LABELS: Record<Priority, string> = {
  critical: 'Critical',
  elevated: 'Elevated',
  normal: 'Normal',
};

const PRIORITIES: Priority[] = ['critical', 'elevated', 'normal'];

const PRIORITY_DOT_CLASS: Record<Exclude<Priority, 'normal'>, string> = {
  critical: 'bg-red-600',
  elevated: 'bg-yellow-300',
};

const SELECT_CLASS =
  'border-line bg-panel text-zinc-200 cursor-pointer appearance-none rounded-full border py-1 pr-7 pl-7 text-xs outline-none disabled:cursor-not-allowed disabled:opacity-60';

const CHEVRON_CLASS =
  'text-muted pointer-events-none absolute top-1/2 right-2 h-3 w-3 -translate-y-1/2';

function PriorityBadge(props: { task: Task; onSave: (priority: Priority) => Promise<void> }) {
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

function SectionBadge(props: {
  task: Task;
  project: ProjectSnapshot;
  onSave: (sectionId: string | null) => Promise<void>;
}) {
  const { task, project, onSave } = props;

  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const section = project.sections.find((candidate) => candidate.id === task.section_id);
  const sectionColor = section?.color ?? null;

  async function change(sectionId: string) {
    const nextSectionId = sectionId.length === 0 ? null : sectionId;
    if (nextSectionId === task.section_id) {
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await onSave(nextSectionId);
    } catch {
      setError('Could not save the section.');
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="flex flex-col gap-1">
      <div className="relative inline-flex">
        <select
          aria-label="Section"
          value={task.section_id ?? ''}
          disabled={saving}
          onChange={(event) => void change(event.target.value)}
          className={SELECT_CLASS}
        >
          <option value="">No section</option>
          {project.sections.map((candidate) => (
            <option key={candidate.id} value={candidate.id}>
              {candidate.name}
            </option>
          ))}
        </select>
        <span
          aria-hidden="true"
          className={`pointer-events-none absolute top-1/2 left-2.5 h-2 w-2 -translate-y-1/2 rounded-full ${
            sectionColor === null ? 'border-line-strong border' : 'ring-1 ring-white/20'
          }`}
          style={
            sectionColor === null ? undefined : { backgroundColor: SECTION_COLOR_HEX[sectionColor] }
          }
        />
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

export type TaskMetaProps = {
  task: Task;
  project: ProjectSnapshot;
  onSavePriority: (priority: Priority) => Promise<void>;
  onSaveSection: (sectionId: string | null) => Promise<void>;
};

export function TaskMeta(props: TaskMetaProps) {
  const { task, project, onSavePriority, onSaveSection } = props;

  return (
    <div className="flex flex-wrap items-start gap-2">
      <PriorityBadge task={task} onSave={onSavePriority} />
      <SectionBadge task={task} project={project} onSave={onSaveSection} />
    </div>
  );
}
