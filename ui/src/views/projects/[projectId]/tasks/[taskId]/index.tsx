import { Alert } from 'flowbite-react';
import { useParams } from 'react-router-dom';
import { useReplicaSnapshot } from '@/providers/Replica';
import { TaskDetail } from './TaskDetail';

export function TaskPage() {
  const { projectId, taskId } = useParams();
  const snapshot = useReplicaSnapshot();

  const project = snapshot.visible.projects.find((candidate) => candidate.id === projectId);
  const task = project?.tasks.find((candidate) => candidate.id === taskId);

  if (project === undefined || task === undefined) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <Alert color="warning" className="max-w-md">
          Task not found. Choose another task from the board.
        </Alert>
      </div>
    );
  }

  return <TaskDetail project={project} task={task} />;
}
