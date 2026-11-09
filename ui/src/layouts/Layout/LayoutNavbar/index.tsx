import { Navbar, NavbarCollapse, NavbarLink } from 'flowbite-react';
import type { NavbarTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { Link } from 'react-router-dom';
import { useActiveProject } from '@/providers/ActiveProject';
import { ConnectionStatus } from './ConnectionStatus';
import { ProjectSwitcher } from './ProjectSwitcher';
import { ScryMark } from './ScryMark';
import { SignOutButton } from './SignOutButton';
import { SyncButton } from './SyncButton';
import { SyncStatus } from './SyncStatus';

export function LayoutNavbar() {
  const { activeProjectId } = useActiveProject();
  const tasksPath = activeProjectId === null ? null : `/projects/${activeProjectId}/tasks`;
  const settingsPath = activeProjectId === null ? null : `/projects/${activeProjectId}/settings`;

  const navbarTheme: DeepPartial<NavbarTheme> = {
    root: {
      base: 'border-b border-line bg-panel px-4 dark:border-line dark:bg-panel',
      inner: {
        base: 'mx-auto flex h-11 flex-wrap items-center justify-between md:h-8',
      },
    },
  };

  return (
    <Navbar fluid theme={navbarTheme}>
      <div className="flex min-w-0 flex-1 items-center gap-2">
        <ScryMark className="h-6 w-6 shrink-0" />
        <ProjectSwitcher />
        <span className="hidden cursor-default text-sm text-zinc-500 select-none md:inline">
          {__APP_VERSION__}
        </span>
      </div>
      <div className="flex flex-1 items-center justify-end gap-2 md:order-2">
        <div className="hidden md:block">
          <ConnectionStatus />
        </div>
        <SyncStatus />
        <div className="hidden md:block">
          <SignOutButton />
        </div>
        <SyncButton />
      </div>
      {tasksPath === null || settingsPath === null ? null : (
        <NavbarCollapse>
          {/* @ts-expect-error polymorphic 'as' prop not typed in flowbite-react */}
          <NavbarLink as={Link} to={tasksPath}>
            Tasks
          </NavbarLink>
          {/* @ts-expect-error polymorphic 'as' prop not typed in flowbite-react */}
          <NavbarLink as={Link} to={settingsPath}>
            Settings
          </NavbarLink>
        </NavbarCollapse>
      )}
    </Navbar>
  );
}
