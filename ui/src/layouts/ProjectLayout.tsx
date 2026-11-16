import { Drawer } from 'flowbite-react';
import { useCallback, useMemo, useState } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router-dom';
import { MIN_HANDLE_LENGTH, shortestHandles } from '@/contract/handles';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { useReplicaSnapshot } from '@/providers/Replica';
import { BoardPage } from '@/views/projects/[projectId]/BoardPage';
import { TaskDetail } from '@/views/projects/[projectId]/tasks/[taskId]/TaskDetail';
import { drawerTheme } from './drawerTheme';

type Panel = { project: ProjectSnapshot; task: Task };

export function ProjectLayout() {
  const { projectId, taskId } = useParams();
  const snapshot = useReplicaSnapshot();
  const navigate = useNavigate();
  const location = useLocation();
  const [lastPanel, setLastPanel] = useState<Panel | null>(null);

  const project = snapshot.visible.projects.find((candidate) => candidate.id === projectId);
  const task = project?.tasks.find((candidate) => candidate.id === taskId);
  const handles = useMemo(
    () =>
      shortestHandles(
        snapshot.visible.projects.flatMap((candidate) => candidate.tasks.map((item) => item.id)),
        MIN_HANDLE_LENGTH,
      ),
    [snapshot],
  );
  const open = project !== undefined && task !== undefined;
  const live = open ? { project, task } : null;
  const panel = live ?? lastPanel;

  const close = useCallback(() => {
    if (project !== undefined && task !== undefined) {
      setLastPanel({ project, task });
    }
    navigate(`/projects/${projectId}/tasks${location.search}`);
  }, [location.search, navigate, project, projectId, task]);

  return (
    <div className="relative flex min-h-0 min-w-0 flex-1">
      <div className="flex min-w-0 flex-1" inert={open}>
        <BoardPage />
      </div>
      <div
        aria-hidden="true"
        onClick={close}
        className={`fixed inset-0 z-30 bg-black/40 transition-opacity duration-300 ${
          open ? 'opacity-100' : 'pointer-events-none opacity-0'
        }`}
      />
      <Drawer
        open={open}
        position="bottom"
        backdrop={false}
        theme={drawerTheme}
        onClose={close}
        aria-hidden={!open}
        inert={!open}
      >
        {panel === null ? null : (
          <TaskDetail
            key={`${panel.task.id}:${open}`}
            project={panel.project}
            task={panel.task}
            handle={handles.get(panel.task.id) ?? panel.task.id}
            onClose={close}
          />
        )}
      </Drawer>
    </div>
  );
}
