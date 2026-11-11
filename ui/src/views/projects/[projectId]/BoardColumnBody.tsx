import type { ComponentProps } from 'react';
import type { Task } from '@/generated/Task';
import { NewTaskForm } from './NewTaskForm';
import { SortableTaskCard } from './SortableTaskCard';
import { TaskCard } from './TaskCard';

export type BoardColumnBodyProps = {
  projectId: string;
  group: string;
  tasks: Task[];
  showPriority: boolean;
  canReorder: boolean;
  showCreate: boolean;
  onCreate?: (title: string) => Promise<void>;
  dropRef: ComponentProps<'div'>['ref'];
  isDropTarget: boolean;
};

export function BoardColumnBody(props: BoardColumnBodyProps) {
  const {
    projectId,
    group,
    tasks,
    showPriority,
    canReorder,
    showCreate,
    onCreate,
    dropRef,
    isDropTarget,
  } = props;

  return (
    <>
      <div
        ref={dropRef}
        className={`flex min-h-10 flex-1 flex-col gap-2 overflow-visible rounded-md pr-1 md:overflow-y-auto ${
          isDropTarget ? 'ring-line-hover ring-1' : ''
        }`}
      >
        {tasks.length === 0
          ? null
          : canReorder
            ? tasks.map((task, index) => (
                <SortableTaskCard
                  key={task.id}
                  projectId={projectId}
                  task={task}
                  index={index}
                  group={group}
                  showPriority={showPriority}
                />
              ))
            : tasks.map((task) => (
                <TaskCard
                  key={task.id}
                  projectId={projectId}
                  task={task}
                  showPriority={showPriority}
                />
              ))}
      </div>
      {onCreate === undefined || !showCreate ? null : <NewTaskForm onCreate={onCreate} />}
    </>
  );
}
