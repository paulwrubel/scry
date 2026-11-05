import { Navbar, NavbarBrand } from 'flowbite-react';
import type { NavbarTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { Link } from 'react-router-dom';
import { ProjectSelect } from './ProjectSelect';
import { SyncButton } from './SyncButton';

export function LayoutNavbar() {
  const navbarTheme: DeepPartial<NavbarTheme> = {
    root: {
      base: 'border-b border-zinc-800 bg-zinc-900 px-4',
    },
  };

  return (
    <Navbar fluid theme={navbarTheme}>
      {/* @ts-expect-error polymorphic 'as' prop not typed in flowbite-react */}
      <NavbarBrand as={Link} to="/">
        <span className="self-center text-xl font-semibold whitespace-nowrap text-white">Scry</span>
      </NavbarBrand>
      <div className="flex items-center gap-2">
        <ProjectSelect />
        <SyncButton />
      </div>
    </Navbar>
  );
}
