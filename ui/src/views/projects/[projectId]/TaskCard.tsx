import { Badge } from 'flowbite-react';
import { useRef } from 'react';
import type { CSSProperties } from 'react';
import { Link } from 'react-router-dom';
import type { Priority } from '@/generated/Priority';
import type { Task } from '@/generated/Task';
import { tagBadgeStyle } from '@/utils/tagColor';

const priorityBadgeColor: Record<Priority, string> = {
  critical: 'priority-critical',
  high: 'priority-high',
  medium: 'priority-medium',
  low: 'priority-low',
  minimal: 'priority-minimal',
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

  const press = useRef<{ x: number; y: number } | null>(null);
  const moved = useRef(false);

  const closed = task.state === 'closed';
  const hasBadges = showPriority || closed || task.tags.length > 0;

  return (
    <div
      ref={nodeRef}
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
      className={`border-line-strong bg-surface hover:border-line-hover flex flex-col gap-1.5 rounded-lg border p-3 select-none ${
        draggable === true ? 'cursor-grab active:cursor-grabbing' : ''
      } ${dragging ? 'opacity-80 shadow-lg' : ''}`}
    >
      <Link
        to={`/projects/${projectId}/tasks/${task.id}`}
        draggable={false}
        className="w-fit max-w-full"
      >
        <span
          className={`text-sm hover:text-blue-400 hover:underline ${
            closed ? 'text-faint line-through' : ''
          }`}
        >
          {task.title}
        </span>
      </Link>
      {hasBadges ? (
        <div className="flex flex-wrap items-center gap-1">
          {showPriority ? (
            <Badge color={priorityBadgeColor[task.priority]}>{task.priority}</Badge>
          ) : null}
          {closed ? <Badge color="gray">Closed</Badge> : null}
          {task.tags.map((tag) => (
            <Badge key={tag} color="gray" style={tagBadgeStyle(tag)}>
              {tag}
            </Badge>
          ))}
        </div>
      ) : null}
    </div>
  );
}
