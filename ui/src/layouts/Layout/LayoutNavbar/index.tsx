import { Navbar, NavbarBrand, NavbarCollapse, NavbarLink, NavbarToggle } from 'flowbite-react';
import type { NavbarTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { Link } from 'react-router-dom';
import { ProjectSelect } from './ProjectSelect';
import { SyncButton } from './SyncButton';

export function LayoutNavbar() {
  const navbarTheme: DeepPartial<NavbarTheme> = {
    root: {
      base: 'border-b border-zinc-800 bg-zinc-900 px-4',
      inner: {
        base: 'mx-auto flex h-16 flex-wrap items-center justify-between',
      },
    },
  };

  return (
    <Navbar fluid theme={navbarTheme}>
      {/* @ts-expect-error polymorphic 'as' prop not typed in flowbite-react */}
      <NavbarBrand as={Link} to="/">
        <span className="self-center text-xl font-semibold whitespace-nowrap text-white">Scry</span>
      </NavbarBrand>
      <div className="flex items-center gap-2 md:order-2">
        <ProjectSelect />
        <SyncButton />
        <NavbarToggle />
      </div>
      <NavbarCollapse>
        {/* @ts-expect-error polymorphic 'as' prop not typed in flowbite-react */}
        <NavbarLink as={Link} to="/projects">
          Projects
        </NavbarLink>
        {/* @ts-expect-error polymorphic 'as' prop not typed in flowbite-react */}
        <NavbarLink as={Link} to="/settings">
          Settings
        </NavbarLink>
        {/* @ts-expect-error polymorphic 'as' prop not typed in flowbite-react */}
        <NavbarLink as={Link} to="/login">
          Sign in
        </NavbarLink>
      </NavbarCollapse>
    </Navbar>
  );
}
