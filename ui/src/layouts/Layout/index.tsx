import { Button, Spinner } from 'flowbite-react';
import { Outlet } from 'react-router-dom';
import { useAuth } from '@/providers/Auth';
import { useReplicaStatus } from '@/providers/Replica';
import { LayoutBottomNav } from './LayoutBottomNav';
import { LayoutNavbar } from './LayoutNavbar';
import { ShellState } from './ShellState';

export function Layout() {
  const { status, error, retry } = useReplicaStatus();
  const { isAuthenticated } = useAuth();

  if (status === 'loading') {
    return (
      <ShellState title="Opening local data" message="Preparing the offline replica.">
        <Spinner size="lg" />
      </ShellState>
    );
  }

  if (status === 'error') {
    return (
      <ShellState title="Could not open local data" message={error?.message ?? 'Unknown error'}>
        <Button color="alternative" onClick={retry}>
          Retry
        </Button>
      </ShellState>
    );
  }

  return (
    <div className="flex h-dvh flex-col pr-[env(safe-area-inset-right)] pl-[env(safe-area-inset-left)]">
      <div className="bg-panel pt-[env(safe-area-inset-top)]">
        <LayoutNavbar />
      </div>
      <main className="bg-canvas relative flex min-h-0 flex-1 flex-col overflow-hidden text-zinc-200">
        <div className="relative z-0 flex min-h-0 flex-1 flex-col">
          <Outlet />
        </div>
      </main>
      {isAuthenticated ? <LayoutBottomNav /> : null}
    </div>
  );
}
