import { useSortable } from '@dnd-kit/react/sortable';
import type { Task } from '@/generated/Task';
import { TaskCard } from './TaskCard';

export type SortableTaskCardProps = {
  projectId: string;
  task: Task;
  showPriority: boolean;
  index: number;
  group: string;
};

export function SortableTaskCard(props: SortableTaskCardProps) {
  const { projectId, task, showPriority, index, group } = props;
  const { ref, isDragging } = useSortable({
    id: task.id,
    index,
    group,
    type: 'item',
    accept: 'item',
  });

  return (
    <TaskCard
      projectId={projectId}
      task={task}
      showPriority={showPriority}
      nodeRef={ref}
      dragging={isDragging}
    />
  );
}
