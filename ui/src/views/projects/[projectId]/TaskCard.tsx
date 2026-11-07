import { Badge } from 'flowbite-react';
import { useRef } from 'react';
import type { CSSProperties } from 'react';
import { HiBars3 } from 'react-icons/hi2';
import { Link, useLocation } from 'react-router-dom';
import type { Priority } from '@/generated/Priority';
import type { Task } from '@/generated/Task';
import { tagBadgeStyle } from '@/utils/tagColor';

const PRIORITY_DOT_CLASS: Record<Priority, string> = {
  critical: 'bg-red-600',
  high: 'bg-orange-500',
  medium: 'bg-yellow-300',
  low: 'bg-purple-500',
  minimal: 'bg-white',
};

export type TaskCardProps = {
  projectId: string;
  task: Task;
  showPriority: boolean;
  nodeRef?: (node: HTMLElement | null) => void;
  style?: CSSProperties;
  dragging?: boolean;
  draggable?: boolean;
  dragHandleRef?: (element: Element | null) => void;
};

export function TaskCard(props: TaskCardProps) {
  const { projectId, task, showPriority, nodeRef, style, dragging, draggable, dragHandleRef } =
    props;

  const location = useLocation();
  const press = useRef<{ x: number; y: number } | null>(null);
  const moved = useRef(false);

  const closed = task.state === 'closed';
  const hasBadges = task.tags.length > 0;

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
      className={`border-line-strong bg-surface hover:border-line-hover flex flex-col gap-1.5 rounded-lg border p-3 select-none ${
        draggable === true ? 'cursor-grab active:cursor-grabbing' : ''
      } ${dragging ? 'opacity-80 shadow-lg' : ''}`}
    >
      <div className="flex min-w-0 items-start gap-2">
        <div className="flex min-w-0 flex-1 flex-col gap-1.5">
          <Link
            to={`/projects/${projectId}/tasks/${task.id}${location.search}`}
            draggable={false}
            className="flex w-fit max-w-full min-w-0 items-center gap-2"
          >
            {showPriority ? (
              <span
                role="img"
                aria-label={`${task.priority} priority`}
                title={task.priority}
                className={`h-2.5 w-2.5 shrink-0 rounded-full ring-1 ring-white/20 ${PRIORITY_DOT_CLASS[task.priority]}`}
              />
            ) : null}
            <span
              className={`min-w-0 text-sm hover:text-blue-400 hover:underline ${
                closed ? 'text-faint line-through' : ''
              }`}
            >
              {task.title}
            </span>
          </Link>
          {hasBadges ? (
            <div className="flex flex-wrap items-center gap-1">
              {task.tags.map((tag) => (
                <Badge key={tag} color="gray" style={tagBadgeStyle(tag)}>
                  {tag}
                </Badge>
              ))}
            </div>
          ) : null}
        </div>
        {dragHandleRef === undefined ? null : (
          <button
            ref={dragHandleRef}
            type="button"
            aria-label={`Drag ${task.title}`}
            title="Drag task"
            className="text-muted flex h-11 w-11 shrink-0 touch-none items-center justify-center rounded-md hover:text-zinc-200 active:cursor-grabbing md:hidden"
          >
            <HiBars3 aria-hidden="true" className="h-5 w-5" />
          </button>
        )}
      </div>
    </div>
  );
}
