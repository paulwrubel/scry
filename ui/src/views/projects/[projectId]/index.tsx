import { Alert, Card } from 'flowbite-react';
import { useParams } from 'react-router-dom';
import { useReplicaSnapshot } from '@/providers/Replica';

export function BoardPage() {
  const { projectId } = useParams();
  const snapshot = useReplicaSnapshot();
  const project = snapshot.visible.projects.find((candidate) => candidate.id === projectId);

  if (project === undefined) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <Alert color="warning" className="max-w-md">
          Project not found. Choose another project from the navbar.
        </Alert>
      </div>
    );
  }

  return (
    <div className="p-6">
      <Card>
        <h1 className="text-2xl font-semibold">{project.name}</h1>
        <p className="text-sm text-zinc-400">No tasks yet.</p>
      </Card>
    </div>
  );
}
