import { Badge } from 'flowbite-react';
import { Link } from 'react-router-dom';
import type { Task } from '@/generated/Task';

type TaskCardProps = {
  projectId: string;
  task: Task;
  showPriority: boolean;
};

export function TaskCard(props: TaskCardProps) {
  const { projectId, task, showPriority } = props;

  const closed = task.state === 'closed';

  return (
    <Link
      to={`/projects/${projectId}/tasks/${task.id}`}
      className="flex flex-col gap-2 rounded-lg border border-zinc-800 bg-zinc-950 p-3 hover:border-zinc-700"
    >
      <span className={closed ? 'text-sm text-zinc-500 line-through' : 'text-sm'}>
        {task.title}
      </span>
      <div className="flex flex-wrap items-center gap-1">
        {showPriority ? <Badge color="info">{task.priority}</Badge> : null}
        {closed ? <Badge color="gray">Closed</Badge> : null}
        {task.tags.map((tag) => (
          <Badge key={tag} color="gray">
            {tag}
          </Badge>
        ))}
      </div>
    </Link>
  );
}
