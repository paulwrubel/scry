import { Badge, Card } from 'flowbite-react';
import type { CardTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import type { Task } from '@/generated/Task';
import { NewTaskForm } from './NewTaskForm';
import { TaskCard } from './TaskCard';

type BoardColumnProps = {
  projectId: string;
  title: string;
  tasks: Task[];
  showPriority: boolean;
  onCreate?: (title: string) => Promise<void>;
};

const columnTheme: DeepPartial<CardTheme> = {
  root: {
    children: 'flex h-full flex-col justify-start gap-2 p-2',
  },
};

export function BoardColumn(props: BoardColumnProps) {
  const { projectId, title, tasks, showPriority, onCreate } = props;

  return (
    <Card theme={columnTheme} className="flex w-96 shrink-0 flex-col">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold">{title}</h2>
        <Badge color="gray">{tasks.length}</Badge>
      </div>
      <div className="flex flex-col gap-2">
        {tasks.length === 0 ? (
          <p className="text-faint text-xs">No tasks.</p>
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
