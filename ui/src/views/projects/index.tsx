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
import { useNavigate } from 'react-router-dom';
import { useActiveProject } from '@/providers/ActiveProject';
import { useReplicaSnapshot } from '@/providers/Replica';

export function ProjectsPage() {
  const snapshot = useReplicaSnapshot();
  const { activeProjectId, setActiveProjectId } = useActiveProject();
  const navigate = useNavigate();

  const projects = snapshot.visible.projects;

  function handleUse(projectId: string) {
    setActiveProjectId(projectId);
    navigate(`/projects/${projectId}`);
  }

  return (
    <div className="flex w-full flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold text-white">Projects</h1>
      <section>
        <h2 className="mb-2 text-lg font-semibold text-zinc-300">Your Projects</h2>
        {projects.length === 0 ? (
          <Alert color="info">No projects yet.</Alert>
        ) : (
          <div className="overflow-x-auto rounded-lg border border-zinc-700">
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
                      <Button
                        size="xs"
                        color="default"
                        disabled={project.id === activeProjectId}
                        onClick={() => handleUse(project.id)}
                      >
                        Use
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </section>
    </div>
  );
}
