import { useContext } from 'react';
import { ActiveProjectContext } from './context';
import type { ActiveProjectContextType } from './context';

export function useActiveProject(): ActiveProjectContextType {
  const context = useContext(ActiveProjectContext);
  if (context === null) {
    throw new Error('useActiveProject must be used within ActiveProjectProvider');
  }
  return context;
}
