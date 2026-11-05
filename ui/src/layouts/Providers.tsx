import { ThemeProvider } from 'flowbite-react';
import { Outlet } from 'react-router-dom';
import { ActiveProjectProvider } from '@/providers/ActiveProject';
import { ReplicaProvider } from '@/providers/Replica';
import { SyncProvider } from '@/providers/Sync';

export function Providers() {
  return (
    <ThemeProvider>
      <ReplicaProvider>
        <SyncProvider>
          <ActiveProjectProvider>
            <Outlet />
          </ActiveProjectProvider>
        </SyncProvider>
      </ReplicaProvider>
    </ThemeProvider>
  );
}
