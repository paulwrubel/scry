import { useState } from 'react';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { SECTION_COLOR_HEX } from '@/utils/sections/color';
import { MetaSelect } from './MetaSelect';

export type SectionBadgeProps = {
  task: Task;
  project: ProjectSnapshot;
  onSave: (sectionId: string | null) => Promise<void>;
};

export function SectionBadge(props: SectionBadgeProps) {
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
    <MetaSelect
      ariaLabel="Section"
      value={task.section_id ?? ''}
      disabled={saving}
      onChange={(value) => void change(value)}
      dot={
        <span
          aria-hidden="true"
          className={`pointer-events-none absolute top-1/2 left-2.5 h-2 w-2 -translate-y-1/2 rounded-full ${
            sectionColor === null ? 'border-line-strong border' : 'ring-1 ring-white/20'
          }`}
          style={
            sectionColor === null ? undefined : { backgroundColor: SECTION_COLOR_HEX[sectionColor] }
          }
        />
      }
      error={error}
    >
      <option value="">No section</option>
      {project.sections.map((candidate) => (
        <option key={candidate.id} value={candidate.id}>
          {candidate.name}
        </option>
      ))}
    </MetaSelect>
  );
}
