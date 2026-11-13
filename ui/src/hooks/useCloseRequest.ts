import { useEffect, useEffectEvent } from 'react';

// TypeScript's DOM lib does not declare CloseWatcher yet, so the slice we use is described
// locally rather than widening the global namespace.
declare class CloseWatcher extends EventTarget {
  constructor(options?: { signal?: AbortSignal });
  requestClose(): void;
  close(): void;
  destroy(): void;
}

// the platform already routes back gestures and Escape to the topmost CloseWatcher, so we avoid
// fake history entries that would change the URL and hijack the back button
export function useCloseRequest(open: boolean, onClose: () => void): void {
  const handleClose = useEffectEvent(onClose);

  useEffect(() => {
    if (!open || typeof CloseWatcher === 'undefined') {
      return;
    }
    const watcher = new CloseWatcher();
    watcher.addEventListener('close', handleClose);
    return () => {
      watcher.removeEventListener('close', handleClose);
      watcher.destroy();
    };
  }, [open]);
}
