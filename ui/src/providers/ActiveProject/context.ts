import { createRequiredContext } from '@/hooks/createRequiredContext';

export type ActiveProjectContextType = {
  activeProjectId: string | null;
  setActiveProjectId: (projectId: string) => void;
  clearActiveProject: () => void;
};

export const [ActiveProjectContext, useActiveProject] =
  createRequiredContext<ActiveProjectContextType>('ActiveProject');
