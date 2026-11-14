import { Navigate, useLocation, useParams } from 'react-router-dom';

export function ProjectTasksRedirect() {
  const { projectId } = useParams();
  const location = useLocation();

  if (projectId === undefined) {
    return <Navigate to="/projects" replace />;
  }

  return <Navigate to={`/projects/${projectId}/tasks${location.search}`} replace />;
}
