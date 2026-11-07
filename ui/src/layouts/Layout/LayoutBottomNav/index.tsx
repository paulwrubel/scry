import { HiClipboardDocumentList, HiCog6Tooth, HiFolder } from 'react-icons/hi2';
import { NavLink } from 'react-router-dom';
import { useActiveProject } from '@/providers/ActiveProject';

export function LayoutBottomNav() {
  const { activeProjectId } = useActiveProject();
  const boardPath = activeProjectId === null ? '/' : `/projects/${activeProjectId}/tasks`;

  return (
    <nav
      aria-label="Primary"
      className="border-line bg-panel border-t pb-[env(safe-area-inset-bottom)] md:hidden"
    >
      <div className="flex">
        <NavLink
          to={boardPath}
          end={activeProjectId === null}
          className={({ isActive }) =>
            `flex min-h-11 flex-1 flex-col items-center justify-center gap-0.5 text-[11px] font-medium ${
              isActive ? 'text-white' : 'text-muted hover:text-zinc-200'
            }`
          }
        >
          <HiClipboardDocumentList aria-hidden="true" className="h-5 w-5" />
          <span>Tasks</span>
        </NavLink>
        <NavLink
          to="/projects"
          end
          className={({ isActive }) =>
            `flex min-h-11 flex-1 flex-col items-center justify-center gap-0.5 text-[11px] font-medium ${
              isActive ? 'text-white' : 'text-muted hover:text-zinc-200'
            }`
          }
        >
          <HiFolder aria-hidden="true" className="h-5 w-5" />
          <span>Projects</span>
        </NavLink>
        <NavLink
          to="/admin"
          end
          className={({ isActive }) =>
            `flex min-h-11 flex-1 flex-col items-center justify-center gap-0.5 text-[11px] font-medium ${
              isActive ? 'text-white' : 'text-muted hover:text-zinc-200'
            }`
          }
        >
          <HiCog6Tooth aria-hidden="true" className="h-5 w-5" />
          <span>Admin</span>
        </NavLink>
      </div>
    </nav>
  );
}
