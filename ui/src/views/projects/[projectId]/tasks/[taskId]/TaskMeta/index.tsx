import type { Priority } from '@/generated/Priority';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { PriorityBadge } from './PriorityBadge';
import { SectionBadge } from './SectionBadge';

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
