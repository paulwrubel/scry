import type { Priority } from '@/generated/Priority';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import type { When } from '@/generated/When';
import { DueBadge } from './DueBadge';
import { PriorityBadge } from './PriorityBadge';
import { SectionBadge } from './SectionBadge';

export type TaskMetaProps = {
  task: Task;
  project: ProjectSnapshot;
  onSavePriority: (priority: Priority) => Promise<void>;
  onSaveSection: (sectionId: string | null) => Promise<void>;
  onSaveDue: (due: When | null) => Promise<void>;
};

export function TaskMeta(props: TaskMetaProps) {
  const { task, project, onSavePriority, onSaveSection, onSaveDue } = props;

  return (
    <div className="flex flex-wrap items-start gap-2">
      <PriorityBadge task={task} onSave={onSavePriority} />
      <SectionBadge task={task} project={project} onSave={onSaveSection} />
      <DueBadge task={task} onSave={onSaveDue} />
    </div>
  );
}
