import { createContext } from 'react';

export type ActiveProjectContextType = {
  activeProjectId: string | null;
  setActiveProjectId: (projectId: string) => void;
  clearActiveProject: () => void;
};

export const ActiveProjectContext = createContext<ActiveProjectContextType | null>(null);
