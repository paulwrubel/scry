import { CollisionPriority } from '@dnd-kit/abstract';
import { useDroppable } from '@dnd-kit/react';
import { Badge, Card } from 'flowbite-react';
import type { CardTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { useState } from 'react';
import { HiLockClosed, HiTrash } from 'react-icons/hi2';
import { RenamableLabel } from '@/components/RenamableLabel';
import type { Color } from '@/generated/Color';
import type { Task } from '@/generated/Task';
import { DeleteSectionModal } from './DeleteSectionModal';
import { NewTaskForm } from './NewTaskForm';
import { SectionColorPicker } from './SectionColorPicker';
import { SortableTaskCard } from './SortableTaskCard';
import { TaskCard } from './TaskCard';

type BoardColumnProps = {
  projectId: string;
  group: string;
  droppable?: boolean;
  title: string;
  color: Color | null;
  tasks: Task[];
  showPriority: boolean;
  canReorder: boolean;
  blockingAutomations?: readonly string[];
  onRename?: (name: string) => void;
  onDelete?: () => void;
  onCreate?: (title: string) => Promise<void>;
  showCreate?: boolean;
  onColorChange?: (color: Color | null) => void;
  nodeRef?: (element: HTMLDivElement | null) => void;
  isDragging?: boolean;
};

const columnTheme: DeepPartial<CardTheme> = {
  root: {
    children: 'flex h-full min-h-0 flex-col justify-start gap-2 p-2',
  },
};

export function BoardColumn(props: BoardColumnProps) {
  const {
    projectId,
    group,
    droppable,
    title,
    color,
    tasks,
    showPriority,
    canReorder,
    blockingAutomations = [],
    onRename,
    onDelete,
    onCreate,
    showCreate = true,
    onColorChange,
    nodeRef,
    isDragging,
  } = props;
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  const { ref, isDropTarget } = useDroppable({
    id: group,
    type: 'column',
    accept: 'item',
    collisionPriority: CollisionPriority.Low,
    disabled: !(droppable ?? true),
  });

  return (
    <>
      <Card
        ref={nodeRef}
        theme={columnTheme}
        className={`flex min-h-0 w-full min-w-0 shrink-0 flex-col md:h-full md:w-96 ${
          isDragging ? 'opacity-80' : ''
        }`}
      >
        <div className="flex shrink-0 items-center justify-between gap-2">
          <div className="flex min-w-0 items-center gap-2">
            {onColorChange === undefined ? null : (
              <SectionColorPicker color={color} onChange={onColorChange} />
            )}
            <RenamableLabel
              label={title}
              onRename={onRename}
              labelAs="h2"
              inputAriaLabel="Section name"
            />
          </div>
          <div className="flex shrink-0 items-center gap-1">
            <Badge color="gray">{tasks.length}</Badge>
            {blockingAutomations.length === 0 ? null : (
              <HiLockClosed
                aria-label="Used by an automation"
                title="This section is used by an automation."
                className="text-muted h-3.5 w-3.5"
              />
            )}
            {onDelete === undefined ? null : (
              <button
                type="button"
                aria-label={`Delete ${title}`}
                title={`Delete ${title}`}
                onClick={() => setConfirmingDelete(true)}
                className="text-muted inline-flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center border-0 bg-transparent p-0 transition-colors hover:text-red-400 focus:outline-none focus-visible:outline-none"
              >
                <HiTrash aria-hidden="true" className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
        </div>
        <div
          ref={ref}
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
      </Card>
      {onDelete === undefined ? null : (
        <DeleteSectionModal
          sectionName={title}
          blockingAutomations={blockingAutomations}
          show={confirmingDelete}
          onClose={() => setConfirmingDelete(false)}
          onDelete={() => {
            setConfirmingDelete(false);
            onDelete();
          }}
        />
      )}
    </>
  );
}
