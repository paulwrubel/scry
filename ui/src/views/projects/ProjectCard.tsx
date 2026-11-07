import { Badge, Button, Card } from 'flowbite-react';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';

export type ProjectCardProps = {
  project: ProjectSnapshot;
  active: boolean;
  onUse: () => void;
  onSettings: () => void;
};

export function ProjectCard(props: ProjectCardProps) {
  const { project, active, onUse, onSettings } = props;

  return (
    <Card>
      <div className="flex min-w-0 items-center gap-2">
        <h3 className="min-w-0 truncate text-base font-semibold text-white" title={project.name}>
          {project.name}
        </h3>
        {active ? (
          <Badge color="success" className="shrink-0">
            Active
          </Badge>
        ) : null}
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
        <Button
          size="sm"
          color="default"
          disabled={active}
          onClick={onUse}
          className="min-h-11 flex-1"
        >
          Use
        </Button>
        <Button size="sm" color="alternative" onClick={onSettings} className="min-h-11 flex-1">
          Settings
        </Button>
      </div>
    </Card>
  );
}
