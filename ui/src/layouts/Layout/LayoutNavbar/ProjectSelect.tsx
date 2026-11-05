import { Label, Select } from 'flowbite-react';
import { useActiveProject } from '@/providers/ActiveProject';
import { useReplicaSnapshot } from '@/providers/Replica';

export function ProjectSelect() {
  const snapshot = useReplicaSnapshot();
  const { activeProjectId, setActiveProjectId } = useActiveProject();

  function handleChange(event: React.ChangeEvent<HTMLSelectElement>) {
    setActiveProjectId(event.target.value);
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
