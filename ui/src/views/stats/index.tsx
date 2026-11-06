import { SyncSection } from './SyncSection';

export function StatsPage() {
  return (
    <div className="flex w-full flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold text-white">Stats</h1>
      <SyncSection />
    </div>
  );
}
