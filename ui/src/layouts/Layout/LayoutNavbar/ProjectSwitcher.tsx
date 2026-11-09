import { Dropdown, DropdownDivider, DropdownItem } from 'flowbite-react';
import { useState } from 'react';
import { HiCheck, HiChevronDown, HiPlus } from 'react-icons/hi2';
import { useLocation, useNavigate } from 'react-router-dom';
import { useActiveProject } from '@/providers/ActiveProject';
import { useReplicaSnapshot } from '@/providers/Replica';
import { NewProjectModal } from '@/views/projects/NewProjectModal';

export function ProjectSwitcher() {
  const { activeProjectId, setActiveProjectId } = useActiveProject();
  const snapshot = useReplicaSnapshot();
  const navigate = useNavigate();
  const location = useLocation();
  const [creating, setCreating] = useState(false);

  const projects = snapshot.visible.projects;
  const active = projects.find((project) => project.id === activeProjectId) ?? null;
  const currentTab = location.pathname.endsWith('/settings') ? 'settings' : 'tasks';

  function selectProject(projectId: string) {
    setActiveProjectId(projectId);
    navigate(`/projects/${projectId}/${currentTab}`);
  }

  function handleCreated(projectId: string) {
    setCreating(false);
    setActiveProjectId(projectId);
    navigate(`/projects/${projectId}/tasks`);
  }

  return (
    <>
      <Dropdown
        inline
        dismissOnClick
        renderTrigger={() => (
          <button
            type="button"
            className="flex min-h-11 min-w-0 items-center gap-1 rounded-lg px-2 text-left text-lg font-semibold text-white transition-colors hover:bg-zinc-800 focus:outline-none md:min-h-0"
          >
            <span className="max-w-[40vw] truncate">{active?.name ?? 'Select project'}</span>
            <HiChevronDown aria-hidden="true" className="h-4 w-4 shrink-0 text-zinc-400" />
          </button>
        )}
      >
        {projects.length === 0 ? (
          <DropdownItem disabled>No projects yet</DropdownItem>
        ) : (
          projects.map((project) => (
            <DropdownItem key={project.id} onClick={() => selectProject(project.id)}>
              <span className="flex min-w-0 items-center justify-between gap-2">
                <span className="truncate">{project.name}</span>
                {project.id === activeProjectId ? (
                  <HiCheck aria-hidden="true" className="h-4 w-4 shrink-0" />
                ) : null}
              </span>
            </DropdownItem>
          ))
        )}
        <DropdownDivider />
        <DropdownItem icon={HiPlus} onClick={() => setCreating(true)}>
          New project
        </DropdownItem>
      </Dropdown>
      {creating ? (
        <NewProjectModal
          existingNames={projects.map((project) => project.name)}
          onClose={() => setCreating(false)}
          onCreated={handleCreated}
        />
      ) : null}
    </>
  );
}
