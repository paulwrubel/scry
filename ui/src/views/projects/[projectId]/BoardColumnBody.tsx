import type { ComponentProps } from 'react';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { Priority } from '@/generated/Priority';
import type { Task } from '@/generated/Task';
import { PRIORITY_BAND_CLASS, PRIORITY_LABELS, PRIORITY_LABEL_CLASS } from '@/utils/priority';
import { NewTaskForm } from './NewTaskForm';
import { SortableTaskCard } from './SortableTaskCard';
import { groupTasksByPriority, withGhostPriority } from './board';

export type BoardColumnBodyProps = {
  projectId: string;
  group: string;
  tasks: Task[];
  showPriority: boolean;
  showCreate: boolean;
  priorityView: boolean;
  ghostPriority: Priority | null;
  dragTarget: { taskId: string; priority: Priority } | null;
  onCreate?: (title: string, externalIdentity: ExternalIdentity | null) => Promise<void>;
  dropRef: ComponentProps<'div'>['ref'];
};

export function BoardColumnBody(props: BoardColumnBodyProps) {
  const {
    projectId,
    group,
    tasks,
    showPriority,
    showCreate,
    priorityView,
    ghostPriority,
    dragTarget,
    onCreate,
    dropRef,
  } = props;

  const bands = groupTasksByPriority(
    tasks,
    dragTarget === null ? undefined : { [dragTarget.taskId]: dragTarget.priority },
  );
  const priorityBands = ghostPriority === null ? bands : withGhostPriority(bands, ghostPriority);

  return (
    <>
      {/* Column drag starts from the header; the list and its scrollbar stay local. */}
      <div
        ref={dropRef}
        data-column-drag-ignore
        data-board-group={group}
        className={`flex min-h-10 flex-1 cursor-default flex-col overflow-visible rounded-md pr-1 md:overflow-y-auto ${
          priorityView ? '-mx-1 -mt-3 gap-0 pt-2' : 'gap-2'
        }`}
      >
        {priorityView
          ? priorityBands.map((band) => {
              const ghost = band.tasks.length === 0;
              const highlighted = dragTarget !== null && band.priority === dragTarget.priority;
              return (
                <div
                  key={band.priority}
                  data-priority-band={band.priority}
                  data-ghost-band={ghost ? band.priority : undefined}
                  className={`relative flex flex-col gap-2 rounded-lg p-1 ring-1 ring-inset ${
                    ghost ? 'min-h-12' : ''
                  } ${
                    highlighted
                      ? PRIORITY_BAND_CLASS[band.priority]
                      : ghost
                        ? 'ring-line'
                        : 'ring-transparent'
                  }`}
                >
                  {highlighted || ghost ? (
                    <span
                      className={`bg-panel absolute -top-2 left-2 rounded px-1.5 py-0.5 text-[10px] leading-none font-medium tracking-wide uppercase ${PRIORITY_LABEL_CLASS[band.priority]}`}
                    >
                      {PRIORITY_LABELS[band.priority]}
                    </span>
                  ) : null}
                  {band.tasks.map((task) => (
                    <SortableTaskCard
                      key={task.id}
                      projectId={projectId}
                      task={task}
                      index={tasks.indexOf(task)}
                      group={group}
                      showPriority={showPriority}
                    />
                  ))}
                </div>
              );
            })
          : tasks.length === 0
            ? null
            : tasks.map((task, index) => (
                <SortableTaskCard
                  key={task.id}
                  projectId={projectId}
                  task={task}
                  index={index}
                  group={group}
                  showPriority={showPriority}
                />
              ))}
      </div>
      {onCreate === undefined || !showCreate ? null : <NewTaskForm onCreate={onCreate} />}
    </>
  );
}
