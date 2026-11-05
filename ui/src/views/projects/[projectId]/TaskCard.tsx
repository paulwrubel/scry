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
  const hasBadges = showPriority || closed || task.tags.length > 0;

  return (
    <Link
      to={`/projects/${projectId}/tasks/${task.id}`}
      className="border-line-strong bg-surface hover:border-line-hover flex flex-col gap-1.5 rounded-lg border p-3"
    >
      <span className={closed ? 'text-faint text-sm line-through' : 'text-sm'}>{task.title}</span>
      {hasBadges ? (
        <div className="flex flex-wrap items-center gap-1">
          {showPriority ? <Badge color="info">{task.priority}</Badge> : null}
          {closed ? <Badge color="gray">Closed</Badge> : null}
          {task.tags.map((tag) => (
            <Badge key={tag} color="gray">
              {tag}
            </Badge>
          ))}
        </div>
      ) : null}
    </Link>
  );
}
