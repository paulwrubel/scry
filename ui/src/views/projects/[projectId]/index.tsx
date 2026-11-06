import { move } from '@dnd-kit/helpers';
import { DragDropProvider, KeyboardSensor, PointerSensor } from '@dnd-kit/react';
import type { DragEndEvent, DragOverEvent } from '@dnd-kit/react';
import { isSortable } from '@dnd-kit/react/sortable';
import { Alert, Select, ToggleSwitch } from 'flowbite-react';
import { useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import type { TaskSortingMode } from '@/generated/TaskSortingMode';
import { useReplica, useReplicaSnapshot } from '@/providers/Replica';
import { updateProjectOperation } from '@/utils/projectOperations';
import { createTaskOperation, moveTaskOperation } from '@/utils/taskOperations';
import { BoardColumn } from './BoardColumn';
import { buildBoard, dropPosition, groupKey, itemsFromGroups, sectionIdFromGroup } from './board';

const SORTING_MODES: { label: string; value: TaskSortingMode }[] = [
  { label: 'Alphabetical', value: 'alphabetical' },
  { label: 'Alphabetical (case-insensitive)', value: 'alphabetical-case-insensitive' },
  { label: 'Creation order', value: 'created' },
  { label: 'Manual', value: 'manual' },
  { label: 'Priority', value: 'priority' },
];

type PendingMove = {
  taskId: string;
  sectionId: string | null;
  position: string;
};

export function BoardPage() {
  const { projectId } = useParams();
  const snapshot = useReplicaSnapshot();
  const replica = useReplica();
  const [showClosed, setShowClosed] = useState(false);
  const [pendingMove, setPendingMove] = useState<PendingMove | null>(null);
  const [dragItems, setDragItems] = useState<Record<string, string[]> | null>(null);

  const found = snapshot.visible.projects.find((candidate) => candidate.id === projectId);

  const optimisticProject = useMemo(() => {
    if (found === undefined || pendingMove === null) {
      return found;
    }
    return applyPendingMove(found, pendingMove);
  }, [found, pendingMove]);

  if (optimisticProject === undefined) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <Alert color="warning" className="max-w-md">
          Project not found. Choose another project from the navbar.
        </Alert>
      </div>
    );
  }

  const project = optimisticProject;
  const canReorder = project.task_sorting_mode === 'manual';
  const baseGroups = buildBoard(project, showClosed);
  const tasksById = new Map(project.tasks.map((task) => [task.id, task]));

  const groups = baseGroups.map((group) => {
    const ids = dragItems?.[groupKey(group.section?.id ?? null)];
    if (ids === undefined) {
      return group;
    }

    return {
      section: group.section,
      tasks: ids.map((id) => tasksById.get(id)).filter((task): task is Task => task !== undefined),
    };
  });

  async function handleCreate(sectionId: string, title: string) {
    await replica.enqueue(createTaskOperation(project, sectionId, title));
  }

  async function handleProjectUpdate(patch: {
    task_sorting_mode?: TaskSortingMode;
    show_priority?: boolean;
  }) {
    const operation = updateProjectOperation(project, {
      name: project.name,
      description: project.description ?? '',
      task_sorting_mode: project.task_sorting_mode,
      show_priority: project.show_priority,
      ...patch,
    });
    if (operation !== null) {
      await replica.enqueue(operation);
    }
  }

  async function handleReorder(task: Task, sectionId: string | null, position: string) {
    const operation = moveTaskOperation(task, sectionId, position);
    if (operation !== null) {
      await replica.enqueue(operation);
    }
  }

  function handleDragStart() {
    if (!canReorder) {
      return;
    }
    setDragItems(itemsFromGroups(baseGroups));
  }

  function handleDragOver(event: DragOverEvent) {
    if (!canReorder || event.operation.source?.type === 'column') {
      return;
    }
    setDragItems((current) => (current === null ? current : move(current, event)));
  }

  function handleDragEnd(event: DragEndEvent) {
    const finalItems = dragItems;
    setDragItems(null);

    if (!canReorder || event.canceled || finalItems === null) {
      return;
    }

    const { source } = event.operation;
    if (
      source === null ||
      !isSortable(source) ||
      source.type !== 'item' ||
      source.group === undefined
    ) {
      return;
    }

    const taskId = String(source.id);
    const key = String(source.group);
    const sectionId = sectionIdFromGroup(key);
    const groupIds = finalItems[key] ?? [];
    const located = groupIds.indexOf(taskId);
    const index = located === -1 ? source.index : located;
    const afterId = groupIds[index + 1] ?? null;
    const position = dropPosition(project, sectionId, taskId, afterId);

    const task = project.tasks.find((candidate) => candidate.id === taskId);
    if (task === undefined) {
      return;
    }

    setPendingMove({ taskId, sectionId, position });
    void handleReorder(task, sectionId, position).finally(() => setPendingMove(null));
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="border-line flex flex-wrap items-center justify-between gap-4 border-b px-6 py-4">
        <div className="flex flex-col gap-1">
          <h1 className="text-2xl font-semibold">{project.name}</h1>
          {project.description === null ? null : (
            <p className="text-muted text-sm">{project.description}</p>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-4">
          <label className="flex items-center gap-2 text-sm">
            <span className="text-muted">Sort</span>
            <Select
              sizing="sm"
              aria-label="Task sorting"
              className="w-56"
              value={project.task_sorting_mode}
              onChange={(event) =>
                void handleProjectUpdate({
                  task_sorting_mode: event.target.value as TaskSortingMode,
                })
              }
            >
              {SORTING_MODES.map((mode) => (
                <option key={mode.value} value={mode.value}>
                  {mode.label}
                </option>
              ))}
            </Select>
          </label>
          <ToggleSwitch
            checked={project.show_priority}
            onChange={(showPriority) => void handleProjectUpdate({ show_priority: showPriority })}
            label="Show priority"
            sizing="sm"
          />
          <ToggleSwitch
            checked={showClosed}
            onChange={setShowClosed}
            label="Show closed"
            sizing="sm"
          />
        </div>
      </div>
      {groups.length === 0 ? (
        <div className="flex flex-1 items-center justify-center p-8">
          <Alert color="info" className="max-w-md">
            Nothing to show yet.
          </Alert>
        </div>
      ) : (
        <DragDropProvider
          // The default sensor ignores links so clicks work; allow the title
          // link to start a card drag too, and let the click guard suppress
          // navigation after an actual drag.
          sensors={[PointerSensor.configure({ preventActivation: () => false }), KeyboardSensor]}
          onDragStart={handleDragStart}
          onDragOver={handleDragOver}
          onDragEnd={handleDragEnd}
        >
          <div className="flex min-h-0 flex-1 gap-4 overflow-x-auto overflow-y-hidden p-6">
            {groups.map((group) => {
              const section = group.section;

              return (
                <BoardColumn
                  key={section?.id ?? 'no-section'}
                  projectId={project.id}
                  sectionId={section?.id ?? null}
                  title={section?.name ?? 'No section'}
                  tasks={group.tasks}
                  showPriority={project.show_priority}
                  canReorder={canReorder}
                  onCreate={
                    section === null ? undefined : (title) => handleCreate(section.id, title)
                  }
                />
              );
            })}
          </div>
        </DragDropProvider>
      )}
    </div>
  );
}

function applyPendingMove(project: ProjectSnapshot, pending: PendingMove): ProjectSnapshot {
  return {
    ...project,
    tasks: project.tasks.map((task) =>
      task.id === pending.taskId
        ? { ...task, section_id: pending.sectionId, position: pending.position }
        : task,
    ),
  };
}
