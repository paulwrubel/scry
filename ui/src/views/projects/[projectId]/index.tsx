import { PointerActivationConstraints } from '@dnd-kit/dom';
import { DragDropProvider, KeyboardSensor, PointerSensor } from '@dnd-kit/react';
import type { DragEndEvent } from '@dnd-kit/react';
import { isSortable } from '@dnd-kit/react/sortable';
import { Alert, Select, ToggleSwitch } from 'flowbite-react';
import { useMemo, useState } from 'react';
import { useParams, useSearchParams } from 'react-router-dom';
import type { Color } from '@/generated/Color';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { Priority } from '@/generated/Priority';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { useCollapsedSections } from '@/hooks/useCollapsedSections';
import { useIsDesktop } from '@/hooks/useMediaQuery';
import type { Task } from '@/generated/Task';
import { useReplica, useReplicaSnapshot } from '@/providers/Replica';
import { updateProjectOperation } from '@/utils/projectOperations';
import {
  createSectionOperation,
  deleteSectionOperation,
  moveSectionOperation,
  setSectionColorOperation,
  updateSectionOperation,
} from '@/utils/sectionOperations';
import {
  applyTaskFilter,
  clearTaskFilter,
  isTaskFilterNarrowing,
  parseTaskFilter,
} from '@/utils/taskFilter';
import type { TaskFilter } from '@/utils/taskFilter';
import { createTaskOperation, moveTaskOperation } from '@/utils/taskOperations';
import { applyTaskSortingMode, parseTaskSortingMode, TASK_SORTING_OPTIONS } from '@/utils/taskView';
import { BoardColumn } from './BoardColumn';
import { MobileAddTask } from './MobileAddTask';
import { NewSectionCard } from './NewSectionCard';
import { SortableBoardColumn, type SectionBoardGroup } from './SortableBoardColumn';
import { TaskFilterPopover } from './TaskFilterPopover';
import {
  CLOSED_NO_SECTION_GROUP,
  OPEN_NO_SECTION_GROUP,
  buildBoard,
  dragPriority,
  dropPosition,
  dropPositionForPriority,
  groupContaining,
  groupKey,
  itemsFromGroups,
  planSectionMove,
  sectionIdFromGroup,
} from './board';
import { useTaskDragPreview } from './useTaskDragPreview';

const defaultActivationConstraints = PointerSensor.defaults.activationConstraints;

type PendingMove = {
  taskId: string;
  sectionId: string | null;
  position: string;
  priority: Priority | null;
};

export function BoardPage() {
  const { projectId } = useParams();
  const snapshot = useReplicaSnapshot();
  const replica = useReplica();
  const [searchParams, setSearchParams] = useSearchParams();
  const [pendingMove, setPendingMove] = useState<PendingMove | null>(null);
  const isDesktop = useIsDesktop();
  const { collapsedSections, toggleCollapsedSection } = useCollapsedSections(projectId ?? '');

  const found = snapshot.visible.projects.find((candidate) => candidate.id === projectId);

  const optimisticProject = useMemo(() => {
    if (found === undefined || pendingMove === null) {
      return found;
    }
    return applyPendingMove(found, pendingMove);
  }, [found, pendingMove]);

  const filter = useMemo(() => parseTaskFilter(searchParams), [searchParams]);
  const sortMode = useMemo(() => parseTaskSortingMode(searchParams), [searchParams]);

  const narrowing = isTaskFilterNarrowing(filter);
  const baseGroups = useMemo(
    () => (optimisticProject === undefined ? [] : buildBoard(optimisticProject, filter, sortMode)),
    [optimisticProject, filter, sortMode],
  );
  const {
    dragItems,
    draggedTaskId,
    boundarySide,
    handleDragStart,
    handleDragMove,
    handleDragOver,
    resetDragPreview,
  } = useTaskDragPreview(baseGroups, sortMode === 'priority');

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
  const tasksById = new Map(project.tasks.map((task) => [task.id, task]));

  const destinationKey =
    draggedTaskId === null || dragItems === null ? null : groupContaining(dragItems, draggedTaskId);

  const draggedTask =
    draggedTaskId === null ? undefined : project.tasks.find((task) => task.id === draggedTaskId);
  const originGroup =
    draggedTask === undefined
      ? undefined
      : baseGroups.find((group) => group.tasks.some((task) => task.id === draggedTask.id));
  const ghostGroupKey = originGroup === undefined ? null : groupKey(originGroup);
  const soloGhostPriority =
    sortMode === 'priority' &&
    draggedTask !== undefined &&
    originGroup !== undefined &&
    originGroup.tasks.filter((task) => task.priority === draggedTask.priority).length === 1
      ? draggedTask.priority
      : null;

  const provisionalPriority =
    dragItems === null || draggedTaskId === null
      ? null
      : dragPriority(
          project,
          sortMode,
          itemsFromGroups(baseGroups),
          dragItems,
          draggedTaskId,
          boundarySide,
        );

  const groups = baseGroups.map((group) => {
    const ids = dragItems?.[groupKey(group)];
    if (ids === undefined) {
      return group;
    }

    return {
      section: group.section,
      state: group.state,
      tasks: ids.map((id) => tasksById.get(id)).filter((task): task is Task => task !== undefined),
    };
  });

  const openNoSectionGroup =
    groups.find((group) => group.section === null && group.state === 'open') ?? null;
  const closedNoSectionGroup =
    groups.find((group) => group.section === null && group.state === 'closed') ?? null;
  const sectionGroups = groups.filter(
    (group): group is SectionBoardGroup => group.section !== null,
  );
  const visibleTaskCount = baseGroups.reduce((total, group) => total + group.tasks.length, 0);

  async function handleCreate(
    sectionId: string | null,
    title: string,
    externalIdentity: ExternalIdentity | null,
  ) {
    await replica.enqueue(createTaskOperation(project, sectionId, title, externalIdentity));
  }

  async function handleProjectUpdate(patch: { show_priority?: boolean }) {
    const operation = updateProjectOperation(project, {
      name: project.name,
      description: project.description ?? '',
      show_priority: project.show_priority,
      ...patch,
    });
    if (operation !== null) {
      await replica.enqueue(operation);
    }
  }

  function handleFilterChange(next: TaskFilter) {
    setSearchParams(applyTaskFilter(searchParams, next), { replace: true });
  }

  function handleFilterClear() {
    setSearchParams(applyTaskFilter(searchParams, clearTaskFilter()), { replace: true });
  }

  async function handleSectionRename(sectionId: string, name: string) {
    const operation = updateSectionOperation(sectionId, name);
    if (operation !== null) {
      await replica.enqueue(operation);
    }
  }

  function handleSectionColorChange(sectionId: string, color: Color | null) {
    void replica.enqueue(setSectionColorOperation(sectionId, color));
  }

  function handleAddSection(name: string) {
    const operation = createSectionOperation(project, name);
    if (operation !== null) {
      void replica.enqueue(operation);
    }
  }

  function handleSectionDelete(sectionId: string) {
    void replica.enqueue(deleteSectionOperation(sectionId));
  }

  function handleSectionMove(sectionId: string, position: string) {
    void replica.enqueue(moveSectionOperation(sectionId, position));
  }

  async function handleReorder(
    task: Task,
    sectionId: string | null,
    position: string,
    priority: Priority | null,
  ) {
    const operation = moveTaskOperation(task, sectionId, position, priority);
    if (operation !== null) {
      await replica.enqueue(operation);
    }
  }

  function handleDragEnd(event: DragEndEvent) {
    const { source } = event.operation;
    const finalItems = dragItems;
    resetDragPreview();

    if (event.canceled) {
      return;
    }

    if (source?.type === 'column') {
      if (!isSortable(source)) {
        return;
      }

      const planned = planSectionMove(
        sectionGroups.map((group) => group.section),
        source.initialIndex,
        source.index,
      );
      if (planned !== null) {
        handleSectionMove(planned.sectionId, planned.position);
      }
      return;
    }

    if (finalItems === null) {
      return;
    }

    if (source === null || !isSortable(source) || source.type !== 'item') {
      return;
    }

    const taskId = String(source.id);
    const key = Object.entries(finalItems).find(([, ids]) => ids.includes(taskId))?.[0];
    if (key === undefined) {
      return;
    }

    const task = project.tasks.find((candidate) => candidate.id === taskId);
    if (task === undefined) {
      return;
    }

    const startingItems = itemsFromGroups(baseGroups);
    const originKey = groupContaining(startingItems, taskId);
    const originIndex = originKey === null ? -1 : startingItems[originKey].indexOf(taskId);
    const finalIndex = (finalItems[key] ?? []).indexOf(taskId);
    const priority = dragPriority(
      project,
      sortMode,
      startingItems,
      finalItems,
      taskId,
      boundarySide,
    );
    // a same-slot drop can still cross a priority boundary, so only bail when
    // both the index and the resolved priority are unchanged
    if (
      key === originKey &&
      finalIndex === originIndex &&
      (priority === null || priority === task.priority)
    ) {
      return;
    }

    const sectionId = sectionIdFromGroup(key);
    const groupIds = finalItems[key] ?? [];
    const located = groupIds.indexOf(taskId);
    const index = located === -1 ? source.index : located;
    const afterId = groupIds[index + 1] ?? null;
    const position =
      priority === null
        ? dropPosition(project, sectionId, taskId, afterId)
        : dropPositionForPriority(project, sectionId, priority, taskId, afterId);

    setPendingMove({ taskId, sectionId, position, priority });
    void handleReorder(task, sectionId, position, priority).finally(() => setPendingMove(null));
  }

  return (
    <div className="flex min-h-0 w-full min-w-0 flex-1 flex-col overflow-y-auto md:overflow-hidden">
      <div className="border-line flex max-w-full min-w-0 flex-wrap items-center gap-x-2 border-b px-4 py-3 md:gap-2 md:px-6 md:py-4">
        <div className="flex shrink-0 items-center gap-2 md:ml-auto md:gap-4">
          <TaskFilterPopover
            filter={filter}
            tasks={project.tasks}
            onChange={handleFilterChange}
            onClear={handleFilterClear}
          />
          <label className="flex items-center gap-2 text-sm">
            <span className="text-muted hidden md:inline">Sort</span>
            <Select
              sizing="sm"
              aria-label="Task sorting"
              className="w-40 md:w-56"
              value={sortMode}
              onChange={(event) => {
                const mode = TASK_SORTING_OPTIONS.find(
                  (option) => option.value === event.target.value,
                );
                if (mode !== undefined) {
                  setSearchParams(applyTaskSortingMode(searchParams, mode.value), {
                    replace: true,
                  });
                }
              }}
            >
              {TASK_SORTING_OPTIONS.map((mode) => (
                <option key={mode.value} value={mode.value}>
                  {mode.label}
                </option>
              ))}
            </Select>
          </label>
          <div className="hidden md:block">
            <ToggleSwitch
              checked={project.show_priority}
              onChange={(showPriority) => void handleProjectUpdate({ show_priority: showPriority })}
              label="Show priority"
              sizing="sm"
              className="h-8 items-center [&>span]:mt-0"
            />
          </div>
        </div>
      </div>
      {narrowing && visibleTaskCount === 0 ? (
        <div className="border-line text-muted flex items-center gap-3 border-b px-4 py-2 text-sm md:px-6">
          <span>No tasks match the active filters.</span>
          <button
            type="button"
            onClick={handleFilterClear}
            className="text-blue-400 transition-colors hover:text-blue-300"
          >
            Clear filters
          </button>
        </div>
      ) : null}
      <DragDropProvider
        sensors={[
          PointerSensor.configure({
            activationConstraints(event, source) {
              if (event.pointerType === 'touch') {
                return [
                  new PointerActivationConstraints.Delay({
                    value: 300,
                    tolerance: 8,
                  }),
                ];
              }

              return typeof defaultActivationConstraints === 'function'
                ? defaultActivationConstraints(event, source)
                : defaultActivationConstraints;
            },
            preventActivation: (event, source) => {
              if (source.type !== 'column') {
                return false;
              }
              const target = event.target;
              return (
                target instanceof Element &&
                target.closest('[data-column-drag-ignore], button, input, textarea, select, a') !==
                  null
              );
            },
          }),
          KeyboardSensor,
        ]}
        onDragStart={handleDragStart}
        onDragMove={handleDragMove}
        onDragOver={handleDragOver}
        onDragEnd={handleDragEnd}
      >
        <div className="flex w-full min-w-0 flex-none flex-col gap-3 p-3 md:min-h-0 md:flex-1 md:flex-row md:gap-4 md:overflow-x-auto md:overflow-y-hidden md:p-6">
          {openNoSectionGroup === null ? null : (
            <BoardColumn
              projectId={project.id}
              group={OPEN_NO_SECTION_GROUP}
              droppable={false}
              title="Open (No Section)"
              color={null}
              tasks={openNoSectionGroup.tasks}
              showPriority={project.show_priority}
              priorityView={sortMode === 'priority'}
              ghostPriority={
                ghostGroupKey === groupKey(openNoSectionGroup) ? soloGhostPriority : null
              }
              dragTarget={
                provisionalPriority !== null &&
                draggedTaskId !== null &&
                groupKey(openNoSectionGroup) === destinationKey
                  ? { taskId: draggedTaskId, priority: provisionalPriority }
                  : null
              }
              showCreate={isDesktop}
              onCreate={(title, identity) => handleCreate(null, title, identity)}
              collapsed={!isDesktop && collapsedSections.has(OPEN_NO_SECTION_GROUP)}
              onToggleCollapsed={
                isDesktop ? undefined : () => toggleCollapsedSection(OPEN_NO_SECTION_GROUP)
              }
            />
          )}
          {sectionGroups.map((group, index) => (
            <SortableBoardColumn
              key={group.section.id}
              group={group}
              index={index}
              project={project}
              sectionsSortable={isDesktop}
              showCreate={isDesktop}
              priorityView={sortMode === 'priority'}
              ghostPriority={ghostGroupKey === groupKey(group) ? soloGhostPriority : null}
              dragTarget={
                provisionalPriority !== null &&
                draggedTaskId !== null &&
                groupKey(group) === destinationKey
                  ? { taskId: draggedTaskId, priority: provisionalPriority }
                  : null
              }
              collapsed={!isDesktop && collapsedSections.has(group.section.id)}
              onToggleCollapsed={
                isDesktop ? undefined : () => toggleCollapsedSection(group.section.id)
              }
              onCreate={handleCreate}
              onRename={(sectionId, name) => void handleSectionRename(sectionId, name)}
              onColorChange={handleSectionColorChange}
              onDelete={handleSectionDelete}
            />
          ))}
          {closedNoSectionGroup === null ? null : (
            <BoardColumn
              projectId={project.id}
              group={CLOSED_NO_SECTION_GROUP}
              droppable={false}
              title="Closed (No Section)"
              color={null}
              tasks={closedNoSectionGroup.tasks}
              showPriority={project.show_priority}
              priorityView={sortMode === 'priority'}
              ghostPriority={
                ghostGroupKey === groupKey(closedNoSectionGroup) ? soloGhostPriority : null
              }
              dragTarget={
                provisionalPriority !== null &&
                draggedTaskId !== null &&
                groupKey(closedNoSectionGroup) === destinationKey
                  ? { taskId: draggedTaskId, priority: provisionalPriority }
                  : null
              }
              collapsed={!isDesktop && collapsedSections.has(CLOSED_NO_SECTION_GROUP)}
              onToggleCollapsed={
                isDesktop ? undefined : () => toggleCollapsedSection(CLOSED_NO_SECTION_GROUP)
              }
            />
          )}
          <NewSectionCard onAdd={handleAddSection} />
        </div>
      </DragDropProvider>
      {isDesktop ? null : (
        <MobileAddTask onCreate={(title, identity) => handleCreate(null, title, identity)} />
      )}
    </div>
  );
}

function applyPendingMove(project: ProjectSnapshot, pending: PendingMove): ProjectSnapshot {
  return {
    ...project,
    tasks: project.tasks.map((task) =>
      task.id === pending.taskId
        ? {
            ...task,
            section_id: pending.sectionId,
            position: pending.position,
            priority: pending.priority ?? task.priority,
          }
        : task,
    ),
  };
}
