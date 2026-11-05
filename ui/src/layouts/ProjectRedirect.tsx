import { Navigate } from 'react-router-dom';
import { useActiveProject } from '@/providers/ActiveProject';

export function ProjectRedirect() {
  const { activeProjectId } = useActiveProject();

  if (activeProjectId === null) {
    return <Navigate to="/projects" replace />;
  }

  return <Navigate to={`/projects/${activeProjectId}`} replace />;
}
