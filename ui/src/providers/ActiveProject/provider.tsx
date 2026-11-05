import { useCallback, useMemo } from 'react';
import { matchPath, useLocation } from 'react-router-dom';
import { useReplicaSnapshot } from '@/providers/Replica';
import { ActiveProjectContext } from './context';
import type { ActiveProjectContextType } from './context';
import {
  readActiveProjectId,
  resolveActiveProjectId,
  writeActiveProjectId,
} from '@/utils/activeProject';

export type ActiveProjectProviderProps = {
  children: React.ReactNode;
};

export function ActiveProjectProvider(props: ActiveProjectProviderProps) {
  const { children } = props;

  const snapshot = useReplicaSnapshot();
  const location = useLocation();
  const projectIds = snapshot.visible.projects.map((project) => project.id);
  const routeProjectId =
    matchPath('/projects/:projectId/*', location.pathname)?.params.projectId ?? null;
  const activeProjectId = resolveActiveProjectId(
    projectIds,
    routeProjectId ?? readActiveProjectId(),
  );

  const setActiveProjectId = useCallback((projectId: string) => {
    writeActiveProjectId(projectId);
  }, []);

  const clearActiveProject = useCallback(() => {
    writeActiveProjectId(null);
  }, []);

  const value = useMemo<ActiveProjectContextType>(
    () => ({ activeProjectId, setActiveProjectId, clearActiveProject }),
    [activeProjectId, setActiveProjectId, clearActiveProject],
  );

  return <ActiveProjectContext.Provider value={value}>{children}</ActiveProjectContext.Provider>;
}
