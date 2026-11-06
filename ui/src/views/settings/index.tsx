import { ProjectSection } from './ProjectSection';
import { SyncSection } from './SyncSection';

export function SettingsPage() {
  return (
    <div className="flex w-full flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold text-white">Settings</h1>
      <SyncSection />
      <ProjectSection />
    </div>
  );
}
