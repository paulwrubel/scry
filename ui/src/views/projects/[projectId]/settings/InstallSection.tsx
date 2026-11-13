import { Button, Card, Modal, ModalBody, ModalFooter, ModalHeader } from 'flowbite-react';
import { useState } from 'react';
import { useCloseRequest } from '@/hooks/useCloseRequest';
import { useInstallPrompt } from '@/providers/InstallPrompt';

export function InstallSection() {
  const { canInstall, isInstalled, isIos, isMobile, promptInstall } = useInstallPrompt();
  const [showInstructions, setShowInstructions] = useState(false);

  function closeInstructions() {
    setShowInstructions(false);
  }

  useCloseRequest(showInstructions, closeInstructions);

  if (isInstalled || (!canInstall && !isIos && !isMobile)) {
    return null;
  }

  const description = canInstall
    ? 'Install Scry for quicker access and a standalone experience.'
    : isIos
      ? 'Add Scry to your home screen to open it like an app.'
      : 'Open your browser menu and choose Install or Add to Home Screen.';

  return (
    <Card>
      <div className="flex flex-col gap-1">
        <h2 className="text-lg font-semibold">Install Scry</h2>
        <p className="text-muted text-sm">{description}</p>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        {canInstall ? (
          <Button color="default" onClick={() => void promptInstall()}>
            Install Scry
          </Button>
        ) : (
          <Button color="alternative" onClick={() => setShowInstructions(true)}>
            Show install steps
          </Button>
        )}
      </div>

      <Modal show={showInstructions} onClose={closeInstructions}>
        <ModalHeader>
          {isIos ? 'Install on iPhone or iPad' : 'Install from your browser'}
        </ModalHeader>
        <ModalBody>
          {isIos ? (
            <ol className="list-decimal space-y-2 pl-5 text-sm text-zinc-300">
              <li>Open Scry in Safari.</li>
              <li>Tap the Share button.</li>
              <li>Choose Add to Home Screen.</li>
              <li>Tap Add.</li>
            </ol>
          ) : (
            <>
              <ol className="list-decimal space-y-2 pl-5 text-sm text-zinc-300">
                <li>Open the browser menu.</li>
                <li>Choose Install, or Add to Home Screen.</li>
                <li>Confirm the installation.</li>
              </ol>
              <p className="text-muted mt-3 text-sm">
                In Firefox, look for Install. If you only see Add to Home Screen, it may create a
                shortcut rather than a standalone app.
              </p>
            </>
          )}
        </ModalBody>
        <ModalFooter className="justify-end">
          <Button color="alternative" onClick={() => setShowInstructions(false)}>
            Close
          </Button>
        </ModalFooter>
      </Modal>
    </Card>
  );
}
