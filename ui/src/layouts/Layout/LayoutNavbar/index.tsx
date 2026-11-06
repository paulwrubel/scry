import { Navbar, NavbarBrand, NavbarCollapse, NavbarLink, NavbarToggle } from 'flowbite-react';
import type { NavbarTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { Link } from 'react-router-dom';
import { useActiveProject } from '@/providers/ActiveProject';
import { SignOutButton } from './SignOutButton';
import { SyncButton } from './SyncButton';
import { SyncStatus } from './SyncStatus';

export function LayoutNavbar() {
  const { activeProjectId } = useActiveProject();
  const boardPath = activeProjectId === null ? '/projects' : `/projects/${activeProjectId}`;

  const navbarTheme: DeepPartial<NavbarTheme> = {
    root: {
      base: 'border-b border-line bg-surface px-4',
      inner: {
        base: 'mx-auto flex h-16 flex-wrap items-center justify-between',
      },
    },
  };

  return (
    <Navbar fluid theme={navbarTheme}>
      <div className="flex flex-1 items-baseline gap-4">
        {/* @ts-expect-error polymorphic 'as' prop not typed in flowbite-react */}
        <NavbarBrand as={Link} to="/">
          <span className="self-center text-xl font-semibold whitespace-nowrap text-white">
            Scry
          </span>
        </NavbarBrand>
        <span className="cursor-default self-baseline text-sm text-zinc-500 select-none">
          {__APP_VERSION__}
        </span>
      </div>
      <div className="flex flex-1 items-center justify-end gap-2 md:order-2">
        <SyncStatus />
        <SignOutButton />
        <SyncButton />
        <NavbarToggle />
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
        <NavbarLink as={Link} to="/settings">
          Settings
        </NavbarLink>
      </NavbarCollapse>
    </Navbar>
  );
}
