import { Alert, Card } from 'flowbite-react';
import { useParams } from 'react-router-dom';
import { useReplicaSnapshot } from '@/providers/Replica';

export function TaskPage() {
  const { projectId, taskId } = useParams();
  const snapshot = useReplicaSnapshot();
  const project = snapshot.visible.projects.find((candidate) => candidate.id === projectId);
  const task = project?.tasks.find((candidate) => candidate.id === taskId);

  if (task === undefined) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <Alert color="warning" className="max-w-md">
          Task not found. Choose another task from the board.
        </Alert>
      </div>
    );
  }

  return (
    <div className="p-6">
      <Card>
        <h1 className="text-2xl font-semibold">{task.title}</h1>
        <p className="text-sm text-zinc-400">Task details arrive with the task editing story.</p>
      </Card>
    </div>
  );
}
