import { Button, Card, Modal, ModalBody, ModalFooter, ModalHeader } from 'flowbite-react';
import { useState } from 'react';
import { useInstallPrompt } from '@/providers/InstallPrompt';

export function InstallSection() {
  const { canInstall, isInstalled, isIos, promptInstall } = useInstallPrompt();
  const [showIosSteps, setShowIosSteps] = useState(false);

  if (isInstalled || (!canInstall && !isIos)) {
    return null;
  }

  return (
    <Card>
      <div className="flex flex-col gap-1">
        <h2 className="text-lg font-semibold">Install Scry</h2>
        <p className="text-muted text-sm">
          {canInstall
            ? 'Install Scry for quicker access and a standalone experience.'
            : 'Add Scry to your home screen to open it like an app.'}
        </p>
      </div>

      {canInstall ? (
        <div className="flex flex-wrap items-center gap-2">
          <Button color="default" onClick={() => void promptInstall()}>
            Install Scry
          </Button>
        </div>
      ) : (
        <div className="flex flex-wrap items-center gap-2">
          <Button color="alternative" onClick={() => setShowIosSteps(true)}>
            Show install steps
          </Button>
        </div>
      )}

      <Modal show={showIosSteps} onClose={() => setShowIosSteps(false)}>
        <ModalHeader>Install on iPhone or iPad</ModalHeader>
        <ModalBody>
          <ol className="list-decimal space-y-2 pl-5 text-sm text-zinc-300">
            <li>Open Scry in Safari.</li>
            <li>Tap the Share button.</li>
            <li>Choose Add to Home Screen.</li>
            <li>Tap Add.</li>
          </ol>
        </ModalBody>
        <ModalFooter>
          <Button color="default" onClick={() => setShowIosSteps(false)}>
            Close
          </Button>
        </ModalFooter>
      </Modal>
    </Card>
  );
}
