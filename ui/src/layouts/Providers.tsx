import { ThemeProvider } from 'flowbite-react';
import { Outlet } from 'react-router-dom';
import { ReplicaProvider } from '@/providers/Replica';

export function Providers() {
  return (
    <ThemeProvider>
      <ReplicaProvider>
        <Outlet />
      </ReplicaProvider>
    </ThemeProvider>
  );
}
