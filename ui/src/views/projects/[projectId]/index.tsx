import { Alert, ToggleSwitch } from 'flowbite-react';
import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { useReplica, useReplicaSnapshot } from '@/providers/Replica';
import { createTaskOperation } from '@/utils/taskOperations';
import { BoardColumn } from './BoardColumn';
import { buildBoard } from './board';

export function BoardPage() {
  const { projectId } = useParams();
  const snapshot = useReplicaSnapshot();
  const replica = useReplica();
  const [showClosed, setShowClosed] = useState(false);

  const found = snapshot.visible.projects.find((candidate) => candidate.id === projectId);

  if (found === undefined) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <Alert color="warning" className="max-w-md">
          Project not found. Choose another project from the navbar.
        </Alert>
      </div>
    );
  }

  const project = found;
  const groups = buildBoard(project, showClosed);

  async function handleCreate(sectionId: string, title: string) {
    await replica.enqueue(createTaskOperation(project, sectionId, title));
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="border-line flex items-center justify-between gap-4 border-b px-6 py-4">
        <div className="flex flex-col gap-1">
          <h1 className="text-2xl font-semibold">{project.name}</h1>
          {project.description === null ? null : (
            <p className="text-muted text-sm">{project.description}</p>
          )}
        </div>
        <ToggleSwitch
          checked={showClosed}
          onChange={setShowClosed}
          label="Show closed"
          sizing="sm"
        />
      </div>
      {groups.length === 0 ? (
        <div className="flex flex-1 items-center justify-center p-8">
          <Alert color="info" className="max-w-md">
            Nothing to show yet.
          </Alert>
        </div>
      ) : (
        <div className="flex min-h-0 flex-1 gap-4 overflow-auto p-6">
          {groups.map((group) => {
            const section = group.section;

            return (
              <BoardColumn
                key={section?.id ?? 'no-section'}
                projectId={project.id}
                title={section?.name ?? 'No section'}
                tasks={group.tasks}
                showPriority={project.show_priority}
                onCreate={section === null ? undefined : (title) => handleCreate(section.id, title)}
              />
            );
          })}
        </div>
      )}
    </div>
  );
}
