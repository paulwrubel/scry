import { Label, Select } from 'flowbite-react';
import { useNavigate } from 'react-router-dom';
import { useActiveProject } from '@/providers/ActiveProject';
import { useReplicaSnapshot } from '@/providers/Replica';

export function ProjectSelect() {
  const snapshot = useReplicaSnapshot();
  const { activeProjectId, setActiveProjectId } = useActiveProject();
  const navigate = useNavigate();

  function handleChange(event: React.ChangeEvent<HTMLSelectElement>) {
    const projectId = event.target.value;
    setActiveProjectId(projectId);
    navigate(`/projects/${projectId}`);
  }

  return (
    <Label className="flex items-center gap-2">
      <span className="text-xs text-zinc-400">Project</span>
      <Select
        sizing="sm"
        value={activeProjectId ?? ''}
        onChange={handleChange}
        disabled={snapshot.visible.projects.length === 0}
      >
        {snapshot.visible.projects.length === 0 ? (
          <option value="">No projects</option>
        ) : (
          snapshot.visible.projects.map((project) => (
            <option key={project.id} value={project.id}>
              {project.name}
            </option>
          ))
        )}
      </Select>
    </Label>
  );
}
