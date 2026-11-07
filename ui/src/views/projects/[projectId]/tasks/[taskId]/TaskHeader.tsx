import { Link, useLocation } from 'react-router-dom';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { TaskActions } from './TaskActions';

export type TaskHeaderProps = {
  project: ProjectSnapshot;
  task: Task;
  handle: string;
  onToggleState: () => Promise<void>;
  onRequestDelete: () => void;
  onBack: () => void;
};

export function TaskHeader(props: TaskHeaderProps) {
  const { project, task, handle, onToggleState, onRequestDelete, onBack } = props;
  const location = useLocation();

  return (
    <div className="flex flex-col gap-3">
      <Link
        to={`/projects/${project.id}${location.search}`}
        aria-label="Back to board"
        title="Back to board"
        className="border-line text-muted hover:border-line-hover inline-flex h-11 w-11 items-center justify-center rounded-lg border transition-colors hover:text-zinc-200 md:h-8 md:w-8"
        onClick={(event) => {
          event.preventDefault();
          onBack();
        }}
      >
        <svg
          aria-hidden="true"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth={1.8}
          className="h-5 w-5 md:h-4 md:w-4"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18"
          />
        </svg>
      </Link>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex flex-col gap-1">
          <div className="flex items-baseline gap-2">
            <h1 className="text-2xl font-semibold">{task.title}</h1>
            <span className="text-faint font-mono text-sm">{handle}</span>
          </div>
          <p className="text-muted text-sm">{project.name}</p>
        </div>
        <TaskActions task={task} onToggleState={onToggleState} onRequestDelete={onRequestDelete} />
      </div>
    </div>
  );
}
