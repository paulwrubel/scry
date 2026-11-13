import type { ComponentProps } from 'react';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { Task } from '@/generated/Task';
import { NewTaskForm } from './NewTaskForm';
import { SortableTaskCard } from './SortableTaskCard';

export type BoardColumnBodyProps = {
  projectId: string;
  group: string;
  tasks: Task[];
  showPriority: boolean;
  showCreate: boolean;
  onCreate?: (title: string, externalIdentity: ExternalIdentity | null) => Promise<void>;
  dropRef: ComponentProps<'div'>['ref'];
};

export function BoardColumnBody(props: BoardColumnBodyProps) {
  const { projectId, group, tasks, showPriority, showCreate, onCreate, dropRef } = props;

  return (
    <>
      {/* Column drag starts from the header; the list and its scrollbar stay local. */}
      <div
        ref={dropRef}
        data-column-drag-ignore
        data-board-group={group}
        className="flex min-h-10 flex-1 cursor-default flex-col gap-2 overflow-visible rounded-md pr-1 md:overflow-y-auto"
      >
        {tasks.length === 0
          ? null
          : tasks.map((task, index) => (
              <SortableTaskCard
                key={task.id}
                projectId={projectId}
                task={task}
                index={index}
                group={group}
                showPriority={showPriority}
              />
            ))}
      </div>
      {onCreate === undefined || !showCreate ? null : <NewTaskForm onCreate={onCreate} />}
    </>
  );
}
