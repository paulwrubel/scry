import { move } from '@dnd-kit/helpers';
import { DragDropProvider, KeyboardSensor, PointerSensor } from '@dnd-kit/react';
import type { DragEndEvent, DragOverEvent, DragStartEvent } from '@dnd-kit/react';
import { isSortable, useSortable } from '@dnd-kit/react/sortable';
import { Alert, Select, ToggleSwitch } from 'flowbite-react';
import { useMemo, useState } from 'react';
import { HiCog6Tooth } from 'react-icons/hi2';
import { useParams, useSearchParams } from 'react-router-dom';
import type { Color } from '@/generated/Color';
import { useIsDesktop } from '@/hooks/useMediaQuery';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import type { TaskSortingMode } from '@/generated/TaskSortingMode';
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
import { ProjectSettingsModal } from '../ProjectSettingsModal';
import { BoardColumn } from './BoardColumn';
import { NewSectionCard } from './NewSectionCard';
import { TaskFilterPopover } from './TaskFilterPopover';
import {
  buildBoard,
  columnDragId,
  dropPosition,
  groupKey,
  itemsFromGroups,
  planSectionMove,
  sectionIdFromGroup,
  type BoardGroup,
} from './board';

const defaultActivationConstraints = PointerSensor.defaults.activationConstraints;

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

type SectionBoardGroup = BoardGroup & { section: Section };

export function BoardPage() {
  const { projectId } = useParams();
  const snapshot = useReplicaSnapshot();
  const replica = useReplica();
  const [searchParams, setSearchParams] = useSearchParams();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [pendingMove, setPendingMove] = useState<PendingMove | null>(null);
  const [dragItems, setDragItems] = useState<Record<string, string[]> | null>(null);
  const isDesktop = useIsDesktop();

  const found = snapshot.visible.projects.find((candidate) => candidate.id === projectId);

  const optimisticProject = useMemo(() => {
    if (found === undefined || pendingMove === null) {
      return found;
    }
    return applyPendingMove(found, pendingMove);
  }, [found, pendingMove]);

  const filter = useMemo(() => parseTaskFilter(searchParams), [searchParams]);

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
  const narrowing = isTaskFilterNarrowing(filter);
  const canReorder = project.task_sorting_mode === 'manual' && !narrowing;
  const baseGroups = buildBoard(project, filter);
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

  const noSectionGroup = groups.find((group) => group.section === null) ?? null;
  const sectionGroups = groups.filter(
    (group): group is SectionBoardGroup => group.section !== null,
  );
  const visibleTaskCount = baseGroups.reduce((total, group) => total + group.tasks.length, 0);
  const dragDisabledReasons: string[] = [];
  if (project.task_sorting_mode !== 'manual') {
    dragDisabledReasons.push('sort mode');
  }
  if (narrowing) {
    dragDisabledReasons.push('applied filters');
  }
  const dragDisabledReason = dragDisabledReasons.join(' and ');

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

  async function handleReorder(task: Task, sectionId: string | null, position: string) {
    const operation = moveTaskOperation(task, sectionId, position);
    if (operation !== null) {
      await replica.enqueue(operation);
    }
  }

  function handleDragStart(event: DragStartEvent) {
    if (!canReorder || event.operation.source?.type === 'column') {
      return;
    }
    setDragItems(itemsFromGroups(baseGroups));
  }

  function handleDragOver(event: DragOverEvent) {
    if (!canReorder || event.operation.source?.type === 'column') {
      return;
    }
    // React owns cross-column moves through dragItems; letting dnd-kit's
    // optimistic sorting relocate the same node races with React's deletion
    // pass and throws removeChild. Section drags keep the plugin (no state).
    event.preventDefault();
    setDragItems((current) => (current === null ? current : move(current, event)));
  }

  function handleDragEnd(event: DragEndEvent) {
    const { source } = event.operation;

    if (event.canceled) {
      setDragItems(null);
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

    const finalItems = dragItems;
    setDragItems(null);

    if (!canReorder || finalItems === null) {
      return;
    }

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
    <div className="flex min-h-0 w-full min-w-0 flex-1 flex-col">
      <div className="border-line flex max-w-full min-w-0 flex-wrap items-center gap-x-2 border-b px-4 py-3 md:gap-2 md:px-6 md:py-4">
        <div className="flex min-w-0 flex-1 items-center gap-2">
          <h1 className="min-w-0 truncate text-2xl font-semibold" title={project.name}>
            {project.name}
          </h1>
          <button
            type="button"
            aria-label="Project settings"
            title="Project settings"
            onClick={() => setSettingsOpen(true)}
            className="text-muted hidden h-11 w-11 shrink-0 cursor-pointer items-center justify-center border-0 bg-transparent p-0 transition-colors hover:text-zinc-200 focus:outline-none focus-visible:outline-none md:inline-flex md:h-8 md:w-8"
          >
            <HiCog6Tooth aria-hidden="true" className="h-5 w-5 md:h-4 md:w-4" />
          </button>
        </div>
        <div className="flex shrink-0 items-center gap-2 md:ml-auto md:gap-4">
          {dragDisabledReason !== '' ? (
            <span className="hidden text-xs text-yellow-500/80 md:inline">
              {`task drag-and-drop disabled due to ${dragDisabledReason}`}
            </span>
          ) : null}
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
        // The default sensor ignores links so clicks work; allow the title
        // link to start a card drag too, and let the click guard suppress
        // navigation after an actual drag.
        sensors={[
          PointerSensor.configure({
            activationConstraints(event, source) {
              if (
                source.handle !== undefined &&
                event.target instanceof Node &&
                source.handle.contains(event.target)
              ) {
                return undefined;
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
        onDragOver={handleDragOver}
        onDragEnd={handleDragEnd}
      >
        <div className="flex min-h-0 w-full min-w-0 flex-1 flex-col gap-3 overflow-y-auto p-3 md:flex-row md:gap-4 md:overflow-x-auto md:overflow-y-hidden md:p-6">
          {noSectionGroup === null ? null : (
            <BoardColumn
              projectId={project.id}
              sectionId={null}
              title="No section"
              color={null}
              tasks={noSectionGroup.tasks}
              showPriority={project.show_priority}
              canReorder={canReorder}
              showDragHandle={!isDesktop}
            />
          )}
          {sectionGroups.map((group, index) => (
            <SortableBoardColumn
              key={group.section.id}
              group={group}
              index={index}
              project={project}
              canReorder={canReorder}
              sectionsSortable={isDesktop}
              showDragHandle={!isDesktop}
              onCreate={handleCreate}
              onRename={(sectionId, name) => void handleSectionRename(sectionId, name)}
              onColorChange={handleSectionColorChange}
              onDelete={handleSectionDelete}
            />
          ))}
          <NewSectionCard onAdd={handleAddSection} />
        </div>
      </DragDropProvider>
      {settingsOpen ? (
        <ProjectSettingsModal
          key={project.id}
          project={project}
          onClose={() => setSettingsOpen(false)}
        />
      ) : null}
    </div>
  );
}

type SortableBoardColumnProps = {
  group: SectionBoardGroup;
  index: number;
  project: ProjectSnapshot;
  canReorder: boolean;
  sectionsSortable: boolean;
  showDragHandle: boolean;
  onCreate: (sectionId: string, title: string) => Promise<void>;
  onRename: (sectionId: string, name: string) => void;
  onColorChange: (sectionId: string, color: Color | null) => void;
  onDelete: (sectionId: string) => void;
};

function SortableBoardColumn(props: SortableBoardColumnProps) {
  const {
    group,
    index,
    project,
    canReorder,
    sectionsSortable,
    showDragHandle,
    onCreate,
    onRename,
    onColorChange,
    onDelete,
  } = props;
  const section = group.section;
  const { ref, isDragging } = useSortable({
    id: columnDragId(section.id),
    index,
    group: 'columns',
    type: 'column',
    accept: 'column',
    disabled: !sectionsSortable,
  });

  return (
    <BoardColumn
      nodeRef={(element) => ref(element)}
      isDragging={isDragging}
      projectId={project.id}
      sectionId={section.id}
      title={section.name}
      color={section.color}
      tasks={group.tasks}
      showPriority={project.show_priority}
      canReorder={canReorder}
      showDragHandle={showDragHandle}
      onRename={(name) => onRename(section.id, name)}
      onColorChange={(color) => onColorChange(section.id, color)}
      onDelete={() => onDelete(section.id)}
      onCreate={(title) => onCreate(section.id, title)}
    />
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
