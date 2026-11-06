import { ThemeProvider } from 'flowbite-react';
import type { CustomFlowbiteTheme } from 'flowbite-react/types';
import { Outlet } from 'react-router-dom';
import { ActiveProjectProvider } from '@/providers/ActiveProject';
import { AuthProvider } from '@/providers/Auth';
import { ReplicaProvider } from '@/providers/Replica';
import { SyncProvider } from '@/providers/Sync';

const theme: CustomFlowbiteTheme = {
  card: {
    root: {
      base: 'flex rounded-lg border border-line bg-panel shadow-md dark:bg-panel',
      children: 'flex h-full flex-col justify-start gap-3 p-4',
    },
  },
  button: {
    base: 'relative flex items-center justify-center rounded-lg text-center font-medium focus:ring-0',
  },
};

export function Providers() {
  return (
    <ThemeProvider theme={theme}>
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
