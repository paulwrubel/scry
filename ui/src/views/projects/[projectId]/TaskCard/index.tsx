import { Badge } from 'flowbite-react';
import { useRef } from 'react';
import type { CSSProperties } from 'react';
import { ClosedReasonIcon } from './ClosedReasonIcon';
import { DueDate } from './DueDate';
import { TaskTitleRow } from './TaskTitleRow';
import type { Task } from '@/generated/Task';
import { externalError } from '@/utils/github';
import { tagBadgeStyle } from '@/utils/tasks/tagColor';

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

  const githubError = externalError(task);
  const closedReason = task.state === 'closed' ? (task.closed_reason ?? null) : null;
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
        <TaskTitleRow projectId={projectId} task={task} showPriority={showPriority} />
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
        {task.due === null && closedReason === null ? null : (
          <div className="flex items-end justify-between gap-2">
            <ClosedReasonIcon reason={closedReason} />
            <DueDate due={task.due} state={task.state} />
          </div>
        )}
      </div>
    </div>
  );
}
