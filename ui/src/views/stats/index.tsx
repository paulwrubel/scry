import { InstallSection } from './InstallSection';
import { SyncSection } from './SyncSection';

export function StatsPage() {
  return (
    <div className="flex min-h-0 w-full flex-1 flex-col gap-6 overflow-y-auto p-4 md:p-6">
      <h1 className="text-2xl font-semibold text-white">Stats</h1>
      <SyncSection />
      <InstallSection />
    </div>
  );
}
