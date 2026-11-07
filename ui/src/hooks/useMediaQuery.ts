import { useCallback, useSyncExternalStore } from 'react';

function matches(query: string): boolean {
  // Tests and non-browser environments have no matchMedia; treat them as desktop.
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
    return true;
  }

  return window.matchMedia(query).matches;
}

export function useMediaQuery(query: string): boolean {
  const subscribe = useCallback(
    (listener: () => void) => {
      if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
        return () => undefined;
      }

      const mediaQuery = window.matchMedia(query);
      mediaQuery.addEventListener('change', listener);
      return () => mediaQuery.removeEventListener('change', listener);
    },
    [query],
  );

  const getSnapshot = useCallback(() => matches(query), [query]);
  const getServerSnapshot = useCallback(() => true, []);

  return useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
}

export function useIsDesktop(): boolean {
  return useMediaQuery('(min-width: 768px)');
}
