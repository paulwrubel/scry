import {
  Alert,
  Button,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeadCell,
  TableRow,
} from 'flowbite-react';
import { useState } from 'react';
import { Navigate, useNavigate } from 'react-router-dom';
import { useActiveProject } from '@/providers/ActiveProject';
import { useReplicaSnapshot } from '@/providers/Replica';
import { formatDate } from '@/utils/date';
import { NewProjectModal } from './NewProjectModal';
import { ProjectCard } from './ProjectCard';

export function ProjectsPage() {
  const snapshot = useReplicaSnapshot();
  const { activeProjectId, setActiveProjectId } = useActiveProject();
  const navigate = useNavigate();
  const [creating, setCreating] = useState(false);

  const projects = snapshot.visible.projects;

  if (activeProjectId !== null) {
    return <Navigate to={`/projects/${activeProjectId}/tasks`} replace />;
  }

  function handleUse(projectId: string) {
    setActiveProjectId(projectId);
    navigate(`/projects/${projectId}/tasks`);
  }

  function handleCreated(projectId: string) {
    setCreating(false);
    setActiveProjectId(projectId);
    navigate(`/projects/${projectId}/tasks`);
  }

  return (
    <div className="flex min-h-0 w-full flex-1 flex-col gap-6 overflow-y-auto p-4 md:p-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold text-white">Projects</h1>
        <Button size="sm" color="default" onClick={() => setCreating(true)}>
          New project
        </Button>
      </div>
      <section>
        <h2 className="mb-2 text-lg font-semibold text-zinc-300">Your Projects</h2>
        {projects.length === 0 ? (
          <Alert color="info">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <span>No projects yet.</span>
              <Button size="sm" color="default" onClick={() => setCreating(true)}>
                New project
              </Button>
            </div>
          </Alert>
        ) : (
          <>
            <div className="flex flex-col gap-3 md:hidden">
              {projects.map((project) => (
                <ProjectCard
                  key={project.id}
                  project={project}
                  onUse={() => handleUse(project.id)}
                />
              ))}
            </div>
            <div className="hidden overflow-x-auto rounded-lg border border-zinc-700 md:block">
              <Table theme={{ root: { base: 'w-full text-left text-sm text-zinc-300' } }}>
                <TableHead>
                  <TableRow>
                    <TableHeadCell>Name</TableHeadCell>
                    <TableHeadCell>Description</TableHeadCell>
                    <TableHeadCell>Sections</TableHeadCell>
                    <TableHeadCell>Tasks</TableHeadCell>
                    <TableHeadCell>Created</TableHeadCell>
                    <TableHeadCell>Actions</TableHeadCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {projects.map((project) => (
                    <TableRow key={project.id}>
                      <TableCell className="font-medium text-white">{project.name}</TableCell>
                      <TableCell className="text-muted">{project.description || '—'}</TableCell>
                      <TableCell>{project.sections.length}</TableCell>
                      <TableCell>{project.tasks.length}</TableCell>
                      <TableCell>{formatDate(project.created_at)}</TableCell>
                      <TableCell>
                        <Button size="xs" color="default" onClick={() => handleUse(project.id)}>
                          Use
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          </>
        )}
      </section>
      {creating ? (
        <NewProjectModal
          existingNames={projects.map((project) => project.name)}
          onClose={() => setCreating(false)}
          onCreated={handleCreated}
        />
      ) : null}
    </div>
  );
}
