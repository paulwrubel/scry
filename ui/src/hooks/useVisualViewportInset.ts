import { useCallback, useSyncExternalStore } from 'react';

function bottomInset(): number {
  const viewport = typeof window === 'undefined' ? null : window.visualViewport;
  if (viewport === null || viewport === undefined) {
    return 0;
  }

  return Math.max(0, window.innerHeight - viewport.offsetTop - viewport.height);
}

export function useVisualViewportInset(): number {
  const subscribe = useCallback((listener: () => void) => {
    const viewport = window.visualViewport;
    if (viewport === null || viewport === undefined) {
      return () => undefined;
    }

    viewport.addEventListener('resize', listener);
    viewport.addEventListener('scroll', listener);
    return () => {
      viewport.removeEventListener('resize', listener);
      viewport.removeEventListener('scroll', listener);
    };
  }, []);

  const getSnapshot = useCallback(() => bottomInset(), []);
  const getServerSnapshot = useCallback(() => 0, []);

  return useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
}
