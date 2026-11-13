import { Alert, Button, Card } from 'flowbite-react';
import { useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useActiveProject } from '@/providers/ActiveProject';
import { useReplicaSnapshot } from '@/providers/Replica';
import { DeleteProjectModal } from '../../DeleteProjectModal';
import { AppSection } from './AppSection';
import { AutomationsSection } from './AutomationsSection';
import { ProjectSettingsForm } from './ProjectSettingsForm';

export function ProjectSettingsPage() {
  const { projectId } = useParams();
  const snapshot = useReplicaSnapshot();
  const { setActiveProjectId, clearActiveProject } = useActiveProject();
  const navigate = useNavigate();
  const [deleteOpen, setDeleteOpen] = useState(false);

  const project = snapshot.visible.projects.find((candidate) => candidate.id === projectId);

  if (project === undefined) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <Alert color="warning" className="max-w-md">
          Project not found. Choose another project from the switcher.
        </Alert>
      </div>
    );
  }

  function handleDeleted(deletedProjectId: string) {
    setDeleteOpen(false);

    const projects = snapshot.visible.projects;
    const index = projects.findIndex((candidate) => candidate.id === deletedProjectId);
    const nextProject =
      index === -1
        ? (projects.find((candidate) => candidate.id !== deletedProjectId) ?? null)
        : (projects[index + 1] ?? projects[index - 1] ?? null);

    if (nextProject === null) {
      clearActiveProject();
      navigate('/projects');
      return;
    }

    setActiveProjectId(nextProject.id);
    navigate(`/projects/${nextProject.id}/tasks`);
  }

  return (
    <div className="flex min-h-0 w-full flex-1 flex-col gap-6 overflow-y-auto p-4 md:p-6">
      <h1 className="text-2xl font-semibold text-white">Project settings</h1>
      <Card>
        <div className="flex flex-col gap-4">
          <h2 className="text-lg font-semibold">Project details</h2>
          <ProjectSettingsForm key={project.id} project={project} />
        </div>
      </Card>
      <Card>
        <div className="flex flex-col gap-3">
          <h2 className="text-lg font-semibold">Automations</h2>
          <p className="text-sm text-zinc-300">Run one action when a task changes.</p>
          <AutomationsSection project={project} />
        </div>
      </Card>
      <Card>
        <div className="flex flex-col gap-4">
          <h2 className="text-lg font-semibold">Danger zone</h2>
          <p className="text-sm text-zinc-300">
            Deleting this project permanently deletes its sections, tasks, comments, and
            automations.
          </p>
          <div>
            <Button color="red" onClick={() => setDeleteOpen(true)}>
              Delete this project
            </Button>
          </div>
        </div>
      </Card>
      <AppSection />
      {deleteOpen ? (
        <DeleteProjectModal
          key={project.id}
          project={project}
          onClose={() => setDeleteOpen(false)}
          onDeleted={handleDeleted}
        />
      ) : null}
    </div>
  );
}
