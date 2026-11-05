import { Link } from 'react-router-dom';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { TaskActions } from './TaskActions';

export type TaskHeaderProps = {
  project: ProjectSnapshot;
  task: Task;
  onToggleState: () => Promise<void>;
  onRequestDelete: () => void;
};

export function TaskHeader(props: TaskHeaderProps) {
  const { project, task, onToggleState, onRequestDelete } = props;

  return (
    <div className="flex flex-col gap-3">
      <Link to={`/projects/${project.id}`} className="text-sm text-zinc-400 hover:text-zinc-200">
        Back to board
      </Link>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex flex-col gap-1">
          <h1 className="text-2xl font-semibold">{task.title}</h1>
          <p className="text-sm text-zinc-400">{project.name}</p>
        </div>
        <TaskActions task={task} onToggleState={onToggleState} onRequestDelete={onRequestDelete} />
      </div>
    </div>
  );
}
