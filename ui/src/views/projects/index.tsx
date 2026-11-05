import { Alert, Card } from 'flowbite-react';
import { Link } from 'react-router-dom';
import { useReplicaSnapshot } from '@/providers/Replica';

export function ProjectsPage() {
  const snapshot = useReplicaSnapshot();
  const projects = snapshot.visible.projects;

  return (
    <div className="flex flex-col gap-4 p-6">
      <h1 className="text-2xl font-semibold">Projects</h1>
      {projects.length === 0 ? (
        <Alert color="info">No projects yet.</Alert>
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {projects.map((project) => (
            <Card key={project.id}>
              <Link to={`/projects/${project.id}`} className="flex flex-col gap-1">
                <span className="text-sm font-medium">{project.name}</span>
                {project.description ? (
                  <span className="text-xs text-zinc-400">{project.description}</span>
                ) : null}
              </Link>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
