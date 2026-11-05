import { useOutlet } from 'react-router-dom';
import { BoardPage } from '@/views/projects/[projectId]';

export function ProjectLayout() {
  const task = useOutlet();

  return (
    <div className="flex min-h-0 flex-1">
      <div className={task === null ? 'flex min-w-0 flex-1' : 'hidden min-w-0 flex-1 md:flex'}>
        <BoardPage />
      </div>
      {task === null ? null : (
        <aside className="flex min-h-0 flex-1 flex-col overflow-auto border-zinc-800 md:w-96 md:shrink-0 md:border-l lg:w-md">
          {task}
        </aside>
      )}
    </div>
  );
}
