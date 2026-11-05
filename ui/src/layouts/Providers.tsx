import { ThemeProvider } from 'flowbite-react';
import { Outlet } from 'react-router-dom';
import { ActiveProjectProvider } from '@/providers/ActiveProject';
import { AuthProvider } from '@/providers/Auth';
import { ReplicaProvider } from '@/providers/Replica';
import { SyncProvider } from '@/providers/Sync';

export function Providers() {
  return (
    <ThemeProvider>
      <ReplicaProvider>
        <AuthProvider>
          <SyncProvider>
            <ActiveProjectProvider>
              <Outlet />
            </ActiveProjectProvider>
          </SyncProvider>
        </AuthProvider>
      </ReplicaProvider>
    </ThemeProvider>
  );
}
