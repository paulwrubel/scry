import { useInstallPrompt } from '@/providers/InstallPrompt';
import { useReplicaSnapshot } from '@/providers/Replica';
import { useSyncSnapshot } from '@/providers/Sync';
import { InstallSection } from './InstallSection';
import { SyncSection } from './SyncSection';

export function AppSection() {
  const replica = useReplicaSnapshot();
  const sync = useSyncSnapshot();
  const { canInstall, isInstalled, isIos, isMobile } = useInstallPrompt();

  const hasSyncDiagnostics = !replica.persisted || sync.rejected.length > 0;
  const hasInstallOption = !isInstalled && (canInstall || isIos || isMobile);

  if (!hasSyncDiagnostics && !hasInstallOption) {
    return null;
  }

  return (
    <section className="flex flex-col gap-3">
      <div>
        <h2 className="text-lg font-semibold text-white">App</h2>
        <p className="text-muted text-sm">Sync diagnostics and install options for this browser.</p>
      </div>
      {hasSyncDiagnostics ? <SyncSection /> : null}
      {hasInstallOption ? <InstallSection /> : null}
    </section>
  );
}
