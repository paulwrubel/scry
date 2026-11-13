import { Badge } from 'flowbite-react';
import { useRef } from 'react';
import type { CSSProperties } from 'react';
import { SiGithub } from 'react-icons/si';
import { VscGitPullRequest, VscIssues } from 'react-icons/vsc';
import { Link, useLocation } from 'react-router-dom';
import type { Priority } from '@/generated/Priority';
import type { Task } from '@/generated/Task';
import { externalError, githubIconColor, githubKind, githubUrl } from '@/utils/external';
import { tagBadgeStyle } from '@/utils/tagColor';

const PRIORITY_DOT_CLASS: Record<Exclude<Priority, 'normal'>, string> = {
  critical: 'bg-red-600',
  elevated: 'bg-yellow-300',
};

export type TaskCardProps = {
  projectId: string;
  task: Task;
  showPriority: boolean;
  nodeRef?: (node: HTMLElement | null) => void;
  style?: CSSProperties;
  dragging?: boolean;
  draggable?: boolean;
};

export function TaskCard(props: TaskCardProps) {
  const { projectId, task, showPriority, nodeRef, style, dragging, draggable } = props;

  const location = useLocation();
  const press = useRef<{ x: number; y: number } | null>(null);
  const moved = useRef(false);

  const closed = task.state === 'closed';
  const url = githubUrl(task);
  const kind = githubKind(task);
  const GitHubIcon = kind === 'pull' ? VscGitPullRequest : kind === 'issue' ? VscIssues : SiGithub;
  const githubLinkLabel =
    kind === 'pull'
      ? 'Open GitHub pull request'
      : kind === 'issue'
        ? 'Open GitHub issue'
        : 'Open GitHub item';
  const githubError = externalError(task);
  const hasBadges = task.tags.length > 0 || githubError !== null;
  const description = task.description?.trim() ?? '';
  const hasDescription = description.length > 0;

  return (
    <div
      ref={nodeRef}
      data-column-drag-ignore
      style={style}
      onPointerDownCapture={(event) => {
        moved.current = false;
        press.current = { x: event.clientX, y: event.clientY };
      }}
      onPointerUpCapture={(event) => {
        if (press.current !== null) {
          const distance = Math.hypot(
            event.clientX - press.current.x,
            event.clientY - press.current.y,
          );
          moved.current = distance > 6;
          press.current = null;
        }
      }}
      onClickCapture={(event) => {
        if (moved.current) {
          event.preventDefault();
          event.stopPropagation();
          moved.current = false;
        }
      }}
      className={`border-line-strong bg-surface hover:border-line-hover flex touch-manipulation flex-col gap-1.5 rounded-lg border p-3 select-none ${
        draggable === true ? 'cursor-grab active:cursor-grabbing' : ''
      } ${dragging ? 'opacity-80 shadow-lg' : ''}`}
    >
      <div className="flex min-w-0 flex-col gap-1.5">
        <div className="flex min-w-0 items-start justify-between gap-2">
          <Link
            to={`/projects/${projectId}/tasks/${task.id}${location.search}`}
            draggable={false}
            className="flex min-w-0 flex-1 items-center gap-2"
          >
            <span
              className={`min-w-0 text-sm wrap-break-word hover:text-blue-400 hover:underline ${
                closed ? 'text-faint line-through' : ''
              }`}
            >
              {task.title}
            </span>
          </Link>
          <div className="mt-1 flex shrink-0 items-center gap-1.5">
            {url === null ? null : (
              <a
                href={url}
                target="_blank"
                rel="noreferrer"
                draggable={false}
                aria-label={githubLinkLabel}
                title={githubLinkLabel}
                className={`${githubIconColor(task)} hover:text-zinc-200`}
                onClick={(event) => event.stopPropagation()}
              >
                <GitHubIcon aria-hidden="true" className="h-3.5 w-3.5" />
              </a>
            )}
            {showPriority && (task.priority === 'critical' || task.priority === 'elevated') ? (
              <span
                role="img"
                aria-label={`${task.priority} priority`}
                title={task.priority}
                className={`h-2 w-2 shrink-0 rounded-full ${PRIORITY_DOT_CLASS[task.priority]}`}
              />
            ) : null}
          </div>
        </div>
        {hasDescription ? (
          <p className="text-faint line-clamp-2 text-xs wrap-break-word whitespace-pre-wrap">
            {description}
          </p>
        ) : null}
        {hasBadges ? (
          <div className="flex flex-wrap items-center gap-1">
            {task.tags.map((tag) => (
              <Badge key={tag} color="gray" style={tagBadgeStyle(tag)}>
                {tag}
              </Badge>
            ))}
            {githubError === null ? null : <Badge color="failure">GitHub: {githubError}</Badge>}
          </div>
        ) : null}
      </div>
    </div>
  );
}
