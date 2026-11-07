import { Navbar, NavbarBrand, NavbarCollapse, NavbarLink } from 'flowbite-react';
import type { NavbarTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { Link } from 'react-router-dom';
import { useActiveProject } from '@/providers/ActiveProject';
import { ConnectionStatus } from './ConnectionStatus';
import { SignOutButton } from './SignOutButton';
import { SyncButton } from './SyncButton';
import { SyncStatus } from './SyncStatus';

export function LayoutNavbar() {
  const { activeProjectId } = useActiveProject();
  const boardPath = activeProjectId === null ? '/projects' : `/projects/${activeProjectId}`;

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
      <div className="flex flex-1 items-baseline gap-4">
        {/* @ts-expect-error polymorphic 'as' prop not typed in flowbite-react */}
        <NavbarBrand as={Link} to="/" className="min-h-11 md:min-h-0">
          <span className="self-center text-xl font-semibold whitespace-nowrap text-white">
            Scry
          </span>
        </NavbarBrand>
        <span className="hidden cursor-default self-baseline text-sm text-zinc-500 select-none md:inline">
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
      <NavbarCollapse>
        {/* @ts-expect-error polymorphic 'as' prop not typed in flowbite-react */}
        <NavbarLink as={Link} to={boardPath}>
          View
        </NavbarLink>
        {/* @ts-expect-error polymorphic 'as' prop not typed in flowbite-react */}
        <NavbarLink as={Link} to="/projects">
          Projects
        </NavbarLink>
        {/* @ts-expect-error polymorphic 'as' prop not typed in flowbite-react */}
        <NavbarLink as={Link} to="/stats">
          Stats
        </NavbarLink>
      </NavbarCollapse>
    </Navbar>
  );
}
