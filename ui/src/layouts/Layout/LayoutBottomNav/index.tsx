import { HiChartBar, HiFolder, HiSquares2X2 } from 'react-icons/hi2';
import { NavLink } from 'react-router-dom';
import { useActiveProject } from '@/providers/ActiveProject';

export function LayoutBottomNav() {
  const { activeProjectId } = useActiveProject();
  const boardPath = activeProjectId === null ? '/' : `/projects/${activeProjectId}`;

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
          <HiSquares2X2 aria-hidden="true" className="h-5 w-5" />
          <span>View</span>
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
          to="/stats"
          end
          className={({ isActive }) =>
            `flex min-h-11 flex-1 flex-col items-center justify-center gap-0.5 text-[11px] font-medium ${
              isActive ? 'text-white' : 'text-muted hover:text-zinc-200'
            }`
          }
        >
          <HiChartBar aria-hidden="true" className="h-5 w-5" />
          <span>Stats</span>
        </NavLink>
      </div>
    </nav>
  );
}
