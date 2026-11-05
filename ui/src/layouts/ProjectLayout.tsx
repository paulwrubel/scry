import { Drawer } from 'flowbite-react';
import type { DrawerTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { useCallback, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { useReplicaSnapshot } from '@/providers/Replica';
import { BoardPage } from '@/views/projects/[projectId]';
import { TaskDetail } from '@/views/projects/[projectId]/tasks/[taskId]/TaskDetail';

type Panel = { project: ProjectSnapshot; task: Task };

const drawerTheme: DeepPartial<DrawerTheme> = {
  root: {
    base: 'fixed z-40 overflow-y-auto bg-surface p-0 transition-transform duration-300 ease-out dark:bg-surface',
    position: {
      right: {
        off: 'right-0 top-0 h-screen w-80 translate-x-[calc(100%_+_1rem)]',
      },
    },
  },
};

export function ProjectLayout() {
  const { projectId, taskId } = useParams();
  const snapshot = useReplicaSnapshot();
  const navigate = useNavigate();
  const [lastPanel, setLastPanel] = useState<Panel | null>(null);

  const project = snapshot.visible.projects.find((candidate) => candidate.id === projectId);
  const task = project?.tasks.find((candidate) => candidate.id === taskId);
  const open = project !== undefined && task !== undefined;
  const live = open ? { project, task } : null;
  const panel = live ?? lastPanel;

  const close = useCallback(() => {
    if (project !== undefined && task !== undefined) {
      setLastPanel({ project, task });
    }
    navigate(`/projects/${projectId}`);
  }, [navigate, project, projectId, task]);

  return (
    <div className="relative flex min-h-0 flex-1">
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
        position="right"
        backdrop={false}
        theme={drawerTheme}
        onClose={close}
        aria-hidden={!open}
        inert={!open}
        className="md:border-line w-full md:inset-y-4 md:right-4 md:h-auto md:w-[36rem] md:rounded-xl md:border md:shadow-2xl"
      >
        {panel === null ? null : (
          <TaskDetail project={panel.project} task={panel.task} onClose={close} />
        )}
      </Drawer>
    </div>
  );
}
