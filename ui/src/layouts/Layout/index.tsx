import { Button, Spinner } from 'flowbite-react';
import { Outlet } from 'react-router-dom';
import { useOnlineStatus } from '@/hooks/useOnlineStatus';
import { ActiveProjectProvider } from '@/providers/ActiveProject';
import { useReplicaStatus } from '@/providers/Replica';
import { LayoutNavbar } from './LayoutNavbar';

export function Layout() {
  const { status, error, retry } = useReplicaStatus();
  const online = useOnlineStatus();

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
    <ActiveProjectProvider>
      <div className="flex h-screen flex-col">
        <LayoutNavbar />
        {!online ? (
          <div className="border-b border-amber-900/50 bg-amber-950/40 px-4 py-2 text-xs text-amber-200">
            Offline. Changes are saved locally and will sync when the connection returns.
          </div>
        ) : null}
        <main className="relative flex min-h-0 flex-1 flex-col overflow-hidden bg-zinc-950 text-zinc-200">
          <div className="relative z-0 flex min-h-0 flex-1 flex-col">
            <Outlet />
          </div>
        </main>
      </div>
    </ActiveProjectProvider>
  );
}

type ShellStateProps = {
  title: string;
  message: string;
  children?: React.ReactNode;
};

function ShellState(props: ShellStateProps) {
  const { title, message, children } = props;

  return (
    <div className="flex min-h-screen flex-col items-center justify-center gap-3 p-8 text-center">
      <h1 className="text-lg font-semibold">{title}</h1>
      <p className="text-sm text-zinc-400">{message}</p>
      {children}
    </div>
  );
}
