import { keyBetween } from '@/contract/positions';
import { compareTasks } from '@/contract/task-order';
import type { Priority } from '@/generated/Priority';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import type { TaskSortingMode } from '@/generated/TaskSortingMode';
import type { TaskState } from '@/generated/TaskState';
import type { TaskFilter } from '@/utils/tasks/filter';
import { matchesTaskFilter } from '@/utils/tasks/filter';

const PRIORITY_ORDER: Priority[] = ['critical', 'elevated', 'normal'];

export type BoardGroup = {
  section: Section | null;
  state: TaskState | null;
  tasks: Task[];
};

export const OPEN_NO_SECTION_GROUP = 'no-section-open';
export const CLOSED_NO_SECTION_GROUP = 'no-section-closed';

export function isNoSectionGroupKey(key: string): boolean {
  return key === OPEN_NO_SECTION_GROUP || key === CLOSED_NO_SECTION_GROUP;
}

export function groupKey(group: Pick<BoardGroup, 'section' | 'state'>): string {
  if (group.section !== null) {
    return group.section.id;
  }
  return group.state === 'closed' ? CLOSED_NO_SECTION_GROUP : OPEN_NO_SECTION_GROUP;
}

export function sectionIdFromGroup(key: string): string | null {
  return isNoSectionGroupKey(key) ? null : key;
}

/** dnd-kit keys its registries by id, so the sortable cannot reuse the id the column's drop target owns. */
export function columnDragId(sectionId: string): string {
  return `column:${sectionId}`;
}

export function buildBoard(
  project: ProjectSnapshot,
  filter: TaskFilter,
  sort: TaskSortingMode,
): BoardGroup[] {
  const visible = project.tasks.filter((task) => matchesTaskFilter(task, filter));

  const sections = [...project.sections].sort((left, right) => {
    if (left.position < right.position) {
      return -1;
    }
    if (left.position > right.position) {
      return 1;
    }
    return 0;
  });

  const groups: BoardGroup[] = sections.map((section) => ({
    section,
    state: null,
    tasks: orderTasks(
      visible.filter((task) => task.section_id === section.id),
      sort,
    ),
  }));

  const openNoSection = orderTasks(
    visible.filter((task) => task.section_id === null && task.state === 'open'),
    sort,
  );

  const closedNoSection = orderTasks(
    visible.filter((task) => task.section_id === null && task.state === 'closed'),
    sort,
  );

  const board: BoardGroup[] = [];

  if (openNoSection.length > 0) {
    board.push({ section: null, state: 'open', tasks: openNoSection });
  }

  board.push(...groups);

  if (closedNoSection.length > 0) {
    board.push({ section: null, state: 'closed', tasks: closedNoSection });
  }

  return board;
}

export function itemsFromGroups(groups: BoardGroup[]): Record<string, string[]> {
  return Object.fromEntries(
    groups.map((group) => [groupKey(group), group.tasks.map((task) => task.id)]),
  );
}

export function groupContaining(items: Record<string, string[]>, taskId: string): string | null {
  for (const [key, ids] of Object.entries(items)) {
    if (ids.includes(taskId)) {
      return key;
    }
  }
  return null;
}

export type TaskSlot = {
  id: string;
  /** Top edge in the column's scroll-content space. */
  top: number;
  /** Bottom edge in the column's scroll-content space. */
  bottom: number;
};

export type DragLayout = {
  groups: Record<string, { gap: number; slots: TaskSlot[] }>;
  /** Height of the card being dragged. */
  draggedHeight: number;
};

/**
 * Returns the insertion index whose placeholder slot center is nearest `anchorY`.
 * `slots` must be the destination's layout with the dragged card removed and
 * the remaining slots shifted up to close its gap.
 */
export function insertionIndexForPointer(
  slots: readonly TaskSlot[],
  anchorY: number,
  draggedHeight: number,
  gap: number,
): number {
  if (slots.length === 0) {
    return 0;
  }

  let bestIndex = 0;
  let bestDistance = Number.POSITIVE_INFINITY;

  for (let index = 0; index <= slots.length; index += 1) {
    const slotTop = index < slots.length ? slots[index].top : slots[slots.length - 1].bottom + gap;
    const slotCenter = slotTop + draggedHeight / 2;
    const distance = Math.abs(slotCenter - anchorY);
    if (distance < bestDistance) {
      bestDistance = distance;
      bestIndex = index;
    }
  }

  return bestIndex;
}

/** Limits preview insertion to the dragged task's own priority peers. */
export type PriorityScope = {
  priority: Priority;
  priorityOf: (id: string) => Priority | null;
};

/**
 * Returns the original `items` when the preview matches it exactly, so callers
 * can skip re-rendering.
 */
export function previewAtPointer(
  items: Record<string, string[]>,
  layout: DragLayout,
  destination: string,
  dragId: string,
  anchorY: number,
  scope?: PriorityScope,
): Record<string, string[]> {
  const withoutDrag: Record<string, string[]> = {};
  for (const [group, ids] of Object.entries(items)) {
    withoutDrag[group] = ids.filter((id) => id !== dragId);
  }

  if (!(destination in items)) {
    return items;
  }

  const groupLayout = layout.groups[destination];
  if (groupLayout === undefined) {
    return items;
  }

  const slots: TaskSlot[] = [];
  let shift = 0;
  for (const slot of groupLayout.slots) {
    if (slot.id === dragId) {
      shift = layout.draggedHeight + groupLayout.gap;
      continue;
    }
    slots.push({ id: slot.id, top: slot.top - shift, bottom: slot.bottom - shift });
  }

  // in priority view the dragged card only competes for slots among its own
  // priority peers, so the preview cannot open a gap in another band
  const scoped =
    scope === undefined
      ? slots
      : slots.filter((slot) => scope.priorityOf(slot.id) === scope.priority);
  const index = insertionIndexForPointer(scoped, anchorY, layout.draggedHeight, groupLayout.gap);

  let insertAt = index;
  if (scope !== undefined) {
    const destinationIds = withoutDrag[destination];
    if (scoped.length > 0) {
      const before = scoped[index];
      if (before !== undefined) {
        insertAt = destinationIds.indexOf(before.id);
      } else {
        insertAt = destinationIds.indexOf(scoped[scoped.length - 1].id) + 1;
      }
    } else {
      const rank = PRIORITY_ORDER.indexOf(scope.priority);
      const at = destinationIds.findIndex((id) => {
        const priority = scope.priorityOf(id);
        return priority !== null && PRIORITY_ORDER.indexOf(priority) > rank;
      });
      insertAt = at === -1 ? destinationIds.length : at;
    }
    if (insertAt === -1) {
      insertAt = destinationIds.length;
    }
  }

  const nextDestination = [...withoutDrag[destination]];
  nextDestination.splice(insertAt, 0, dragId);

  const next: Record<string, string[]> = { ...withoutDrag, [destination]: nextDestination };

  const currentDestination = items[destination];
  const unchanged =
    currentDestination.length === nextDestination.length &&
    currentDestination.every((id, position) => id === nextDestination[position]);

  return unchanged ? items : next;
}

export function sectionTasks(project: ProjectSnapshot, sectionId: string | null): Task[] {
  return project.tasks
    .filter((task) => task.section_id === sectionId)
    .sort((left, right) => compareTasks('manual', left, right));
}

export function insertPosition(tasks: readonly Task[], index: number): string {
  const at = Math.max(0, Math.min(index, tasks.length));
  const before = tasks[at - 1]?.position ?? null;
  const after = tasks[at]?.position ?? null;
  return keyBetween(before, after);
}

export type SectionDrop = { sectionId: string; position: string };

/**
 * Plans where a dragged section lands. `from` is the index the drag started at
 * and `to` is the index dnd-kit projected by the time the drag ended.
 */
export function planSectionMove(
  sections: readonly Section[],
  from: number,
  to: number,
): SectionDrop | null {
  const moved = sections[from];
  if (moved === undefined || from === to || to < 0 || to >= sections.length) {
    return null;
  }

  const reordered = sections.filter((_, index) => index !== from);
  reordered.splice(to, 0, moved);

  const before = reordered[to - 1]?.position ?? null;
  const after = reordered[to + 1]?.position ?? null;
  return { sectionId: moved.id, position: keyBetween(before, after) };
}

export function dropPosition(
  project: ProjectSnapshot,
  sectionId: string | null,
  taskId: string,
  afterId: string | null,
): string {
  const tasks = sectionTasks(project, sectionId).filter((task) => task.id !== taskId);
  const at = afterId === null ? tasks.length : tasks.findIndex((task) => task.id === afterId);
  return insertPosition(tasks, at === -1 ? tasks.length : at);
}

export function dropPositionForPriority(
  project: ProjectSnapshot,
  sectionId: string | null,
  priority: Priority,
  taskId: string,
  afterId: string | null,
): string {
  const tasks = sectionTasks(project, sectionId).filter(
    (task) => task.priority === priority && task.id !== taskId,
  );
  const at = afterId === null ? tasks.length : tasks.findIndex((task) => task.id === afterId);
  return insertPosition(tasks, at === -1 ? tasks.length : at);
}

function orderTasks(tasks: Task[], mode: TaskSortingMode): Task[] {
  return tasks.sort((left, right) => compareTasks(mode, left, right));
}
