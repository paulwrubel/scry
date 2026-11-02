import { Outlet } from 'react-router-dom';

export function Layout() {
  return (
    <div className="flex min-h-screen flex-col">
      <header className="border-b border-zinc-800 px-4 py-3">
        <span className="text-sm font-semibold">Scry</span>
      </header>
      <main className="flex flex-1 flex-col">
        <Outlet />
      </main>
    </div>
  );
}
