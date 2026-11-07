import { Drawer } from 'flowbite-react';
import type { DrawerTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { useCallback, useMemo, useState } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router-dom';
import { MIN_HANDLE_LENGTH, shortestHandles } from '@/contract/handles';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { useReplicaSnapshot } from '@/providers/Replica';
import { BoardPage } from '@/views/projects/[projectId]';
import { TaskDetail } from '@/views/projects/[projectId]/tasks/[taskId]/TaskDetail';

type Panel = { project: ProjectSnapshot; task: Task };

const mobileSheetClasses =
  'inset-x-0 bottom-0 h-[75dvh] w-full rounded-t-xl border-t pb-[env(safe-area-inset-bottom)]';
const desktopDrawerClasses =
  'md:inset-y-4 md:right-4 md:left-auto md:h-auto md:w-xl md:rounded-xl md:border md:pb-0 md:shadow-2xl';

const drawerTheme: DeepPartial<DrawerTheme> = {
  root: {
    base: 'fixed z-40 overflow-y-auto border-line bg-surface p-0 transition-transform duration-300 ease-out',
    // Flowbite accepts one Drawer position per render, so the bottom sheet
    // restores the desktop right-drawer geometry at the md breakpoint.
    position: {
      bottom: {
        on: `${mobileSheetClasses} ${desktopDrawerClasses}`,
        off: `${mobileSheetClasses} ${desktopDrawerClasses} translate-y-full md:translate-y-0 md:translate-x-[calc(100%_+_1rem)]`,
      },
    },
  },
};

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
    navigate(`/projects/${projectId}${location.search}`);
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
