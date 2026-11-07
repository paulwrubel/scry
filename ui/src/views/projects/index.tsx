import {
  Alert,
  Badge,
  Button,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeadCell,
  TableRow,
} from 'flowbite-react';
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { useActiveProject } from '@/providers/ActiveProject';
import { useReplicaSnapshot } from '@/providers/Replica';
import { DeleteProjectModal } from './DeleteProjectModal';
import { ProjectCard } from './ProjectCard';
import { ProjectSettingsModal } from './ProjectSettingsModal';

export function ProjectsPage() {
  const snapshot = useReplicaSnapshot();
  const { activeProjectId, setActiveProjectId, clearActiveProject } = useActiveProject();
  const navigate = useNavigate();
  const [settingsProject, setSettingsProject] = useState<ProjectSnapshot | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<ProjectSnapshot | null>(null);

  const projects = snapshot.visible.projects;

  function handleUse(projectId: string) {
    setActiveProjectId(projectId);
    navigate(`/projects/${projectId}`);
  }

  function handleDeleted(projectId: string) {
    setDeleteTarget(null);
    if (projectId !== activeProjectId) {
      return;
    }

    const index = projects.findIndex((project) => project.id === projectId);
    const nextProject =
      index === -1
        ? (projects.find((project) => project.id !== projectId) ?? null)
        : (projects[index + 1] ?? projects[index - 1] ?? null);

    if (nextProject === null) {
      clearActiveProject();
      navigate('/projects');
      return;
    }

    setActiveProjectId(nextProject.id);
    navigate(`/projects/${nextProject.id}`);
  }

  return (
    <div className="flex min-h-0 w-full flex-1 flex-col gap-6 overflow-y-auto p-4 md:p-6">
      <h1 className="text-2xl font-semibold text-white">Projects</h1>
      <section>
        <h2 className="mb-2 text-lg font-semibold text-zinc-300">Your Projects</h2>
        {projects.length === 0 ? (
          <Alert color="info">No projects yet.</Alert>
        ) : (
          <>
            <div className="flex flex-col gap-3 md:hidden">
              {projects.map((project) => (
                <ProjectCard
                  key={project.id}
                  project={project}
                  active={project.id === activeProjectId}
                  onUse={() => handleUse(project.id)}
                  onSettings={() => setSettingsProject(project)}
                  onDelete={() => setDeleteTarget(project)}
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
                      <TableCell className="font-medium text-white">
                        <div className="flex items-center gap-2">
                          {project.name}
                          {project.id === activeProjectId ? (
                            <Badge color="success">Active</Badge>
                          ) : null}
                        </div>
                      </TableCell>
                      <TableCell className="text-muted">{project.description || '—'}</TableCell>
                      <TableCell>{project.sections.length}</TableCell>
                      <TableCell>{project.tasks.length}</TableCell>
                      <TableCell>{new Date(project.created_at).toLocaleDateString()}</TableCell>
                      <TableCell>
                        <div className="flex flex-wrap items-center gap-2">
                          <Button
                            size="xs"
                            color="default"
                            disabled={project.id === activeProjectId}
                            onClick={() => handleUse(project.id)}
                          >
                            Use
                          </Button>
                          <Button
                            size="xs"
                            color="alternative"
                            onClick={() => setSettingsProject(project)}
                          >
                            Settings
                          </Button>
                          <Button size="xs" color="red" onClick={() => setDeleteTarget(project)}>
                            Delete
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          </>
        )}
      </section>
      {settingsProject === null ? null : (
        <ProjectSettingsModal
          key={settingsProject.id}
          project={settingsProject}
          onClose={() => setSettingsProject(null)}
        />
      )}
      {deleteTarget === null ? null : (
        <DeleteProjectModal
          key={deleteTarget.id}
          project={deleteTarget}
          onClose={() => setDeleteTarget(null)}
          onDeleted={handleDeleted}
        />
      )}
    </div>
  );
}
