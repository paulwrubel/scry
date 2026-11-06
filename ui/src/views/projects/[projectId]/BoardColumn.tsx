import { CollisionPriority } from '@dnd-kit/abstract';
import { useDroppable } from '@dnd-kit/react';
import { Badge, Card } from 'flowbite-react';
import type { CardTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { RenamableLabel } from '@/components/RenamableLabel';
import type { Color } from '@/generated/Color';
import type { Task } from '@/generated/Task';
import { NewTaskForm } from './NewTaskForm';
import { SortableTaskCard } from './SortableTaskCard';
import { TaskCard } from './TaskCard';
import { groupKey } from './board';

type BoardColumnProps = {
  projectId: string;
  sectionId: string | null;
  title: string;
  color: Color | null;
  tasks: Task[];
  showPriority: boolean;
  canReorder: boolean;
  onRename?: (name: string) => void;
  onCreate?: (title: string) => Promise<void>;
};

const SECTION_COLOR_HEX: Record<Color, string> = {
  black: '#000000',
  red: '#ef4444',
  green: '#22c55e',
  yellow: '#eab308',
  blue: '#3b82f6',
  magenta: '#d946ef',
  cyan: '#06b6d4',
  gray: '#9ca3af',
  'dark-gray': '#4b5563',
  'light-red': '#fca5a5',
  'light-green': '#86efac',
  'light-yellow': '#fde047',
  'light-blue': '#93c5fd',
  'light-magenta': '#f0abfc',
  'light-cyan': '#67e8f9',
  white: '#ffffff',
};

const columnTheme: DeepPartial<CardTheme> = {
  root: {
    children: 'flex h-full min-h-0 flex-col justify-start gap-2 p-2',
  },
};

export function BoardColumn(props: BoardColumnProps) {
  const {
    projectId,
    sectionId,
    title,
    color,
    tasks,
    showPriority,
    canReorder,
    onRename,
    onCreate,
  } = props;
  const group = groupKey(sectionId);

  const { ref, isDropTarget } = useDroppable({
    id: group,
    type: 'column',
    accept: 'item',
    collisionPriority: CollisionPriority.Low,
  });

  return (
    <Card
      theme={columnTheme}
      className={`flex h-full min-h-0 w-96 shrink-0 flex-col ${sectionId === null ? 'dark:bg-gray-900' : ''}`}
    >
      <div className="flex shrink-0 items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          {color === null ? null : (
            <span
              aria-hidden="true"
              className="h-2.5 w-2.5 shrink-0 rounded-full ring-1 ring-white/20"
              style={{ backgroundColor: SECTION_COLOR_HEX[color] }}
            />
          )}
          <RenamableLabel
            label={title}
            onRename={onRename}
            labelAs="h2"
            inputAriaLabel="Section name"
          />
        </div>
        <Badge color="gray">{tasks.length}</Badge>
      </div>
      <div
        ref={ref}
        className={`flex min-h-10 flex-1 flex-col gap-2 overflow-y-auto rounded-md pr-1 ${
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
      {onCreate === undefined ? null : <NewTaskForm onCreate={onCreate} />}
    </Card>
  );
}
