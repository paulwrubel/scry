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
      children: 'flex h-full flex-col justify-start gap-3 p-4',
    },
  },
  button: {
    base: 'relative flex items-center justify-center rounded-lg text-center font-medium focus:ring-0',
  },
  badge: {
    root: {
      color: {
        'priority-critical': 'bg-red-600 text-white',
        'priority-high': 'bg-orange-500 text-white',
        'priority-medium': 'bg-yellow-300 text-yellow-950',
        'priority-low': 'bg-sky-300 text-sky-950',
        'priority-minimal': 'bg-purple-200 text-purple-900',
      },
    },
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
