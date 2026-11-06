import { CollisionPriority } from '@dnd-kit/abstract';
import { useDroppable } from '@dnd-kit/react';
import { Badge, Card } from 'flowbite-react';
import type { CardTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import type { Task } from '@/generated/Task';
import { NewTaskForm } from './NewTaskForm';
import { SortableTaskCard } from './SortableTaskCard';
import { TaskCard } from './TaskCard';
import { groupKey } from './board';

type BoardColumnProps = {
  projectId: string;
  sectionId: string | null;
  title: string;
  tasks: Task[];
  showPriority: boolean;
  canReorder: boolean;
  onCreate?: (title: string) => Promise<void>;
};

const columnTheme: DeepPartial<CardTheme> = {
  root: {
    children: 'flex h-full flex-col justify-start gap-2 p-2',
  },
};

export function BoardColumn(props: BoardColumnProps) {
  const { projectId, sectionId, title, tasks, showPriority, canReorder, onCreate } = props;
  const group = groupKey(sectionId);

  const { ref, isDropTarget } = useDroppable({
    id: group,
    type: 'column',
    accept: 'item',
    collisionPriority: CollisionPriority.Low,
  });

  return (
    <Card theme={columnTheme} className="flex w-96 shrink-0 flex-col">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold">{title}</h2>
        <Badge color="gray">{tasks.length}</Badge>
      </div>
      <div
        ref={ref}
        className={`flex min-h-10 flex-1 flex-col gap-2 rounded-md ${
          isDropTarget ? 'ring-line-hover ring-1' : ''
        }`}
      >
        {tasks.length === 0 ? (
          <p className="text-faint text-xs">No tasks.</p>
        ) : canReorder ? (
          tasks.map((task, index) => (
            <SortableTaskCard
              key={task.id}
              projectId={projectId}
              task={task}
              index={index}
              group={group}
              showPriority={showPriority}
            />
          ))
        ) : (
          tasks.map((task) => (
            <TaskCard key={task.id} projectId={projectId} task={task} showPriority={showPriority} />
          ))
        )}
      </div>
      {onCreate === undefined ? null : <NewTaskForm onCreate={onCreate} />}
    </Card>
  );
}
