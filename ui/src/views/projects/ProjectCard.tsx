import { Button, Card } from 'flowbite-react';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';

export type ProjectCardProps = {
  project: ProjectSnapshot;
  onUse: () => void;
};

export function ProjectCard(props: ProjectCardProps) {
  const { project, onUse } = props;

  return (
    <Card>
      <div className="flex min-w-0 items-center gap-2">
        <h3 className="min-w-0 truncate text-base font-semibold text-white" title={project.name}>
          {project.name}
        </h3>
      </div>

      <p className="text-muted text-sm">{project.description || 'No description.'}</p>

      <dl className="grid grid-cols-3 gap-2 text-sm">
        <div>
          <dt className="text-muted text-xs">Sections</dt>
          <dd>{project.sections.length}</dd>
        </div>
        <div>
          <dt className="text-muted text-xs">Tasks</dt>
          <dd>{project.tasks.length}</dd>
        </div>
        <div>
          <dt className="text-muted text-xs">Created</dt>
          <dd>{new Date(project.created_at).toLocaleDateString()}</dd>
        </div>
      </dl>

      <div className="flex gap-2">
        <Button size="sm" color="default" onClick={onUse} className="min-h-11 flex-1">
          Use
        </Button>
      </div>
    </Card>
  );
}
