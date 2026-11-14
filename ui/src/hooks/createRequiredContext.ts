import { createContext, useContext } from 'react';

/**
 * Creates a context whose accessor throws when no provider is mounted, so
 * consumers do not repeat the same null guard for every provider.
 */
export function createRequiredContext<T>(name: string) {
  const Context = createContext<T | null>(null);

  function useRequiredContext(): T {
    const value = useContext(Context);
    if (value === null) {
      throw new Error(`use${name} must be used within ${name}Provider`);
    }
    return value;
  }

  return [Context, useRequiredContext] as const;
}
