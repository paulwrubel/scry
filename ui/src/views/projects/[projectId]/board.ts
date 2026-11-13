import { keyBetween } from '@/contract/positions';
import { compareTasks } from '@/contract/task-order';
import type { Priority } from '@/generated/Priority';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import type { TaskSortingMode } from '@/generated/TaskSortingMode';
import type { TaskState } from '@/generated/TaskState';
import type { TaskFilter } from '@/utils/taskFilter';
import { matchesTaskFilter } from '@/utils/taskFilter';
import { PRIORITIES } from '@/utils/priority';

export type BoardGroup = {
  section: Section | null;
  state: TaskState | null;
  tasks: Task[];
};

export type PriorityGroup = {
  priority: Priority;
  tasks: Task[];
};

/**
 * Adds an empty band for `priority` at its sorted position when the dragged
 * card was the only member of that band and left the column during the drag.
 */
export function withGhostPriority(bands: PriorityGroup[], priority: Priority): PriorityGroup[] {
  if (bands.some((band) => band.priority === priority)) {
    return bands;
  }
  const ghost = { priority, tasks: [] };
  const at = bands.findIndex(
    (band) => PRIORITIES.indexOf(band.priority) > PRIORITIES.indexOf(priority),
  );
  return at === -1 ? [...bands, ghost] : [...bands.slice(0, at), ghost, ...bands.slice(at)];
}

export function groupTasksByPriority(
  tasks: readonly Task[],
  priorityOverrides: Readonly<Record<string, Priority>> = {},
): PriorityGroup[] {
  const groups: PriorityGroup[] = [];
  for (const task of tasks) {
    const priority = priorityOverrides[task.id] ?? task.priority;
    const current = groups[groups.length - 1];
    if (current !== undefined && current.priority === priority) {
      current.tasks.push(task);
      continue;
    }
    groups.push({ priority, tasks: [task] });
  }
  return groups;
}

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

export type BoundarySide = 'above' | 'below';

// a shared boundary is one insertion slot with two meanings, so resolve it by
// which side of the visual gap midpoint the pointer is on. `slots` is the
// collapsed insertion layout, so the dragged card's own height no longer shifts
// the threshold
// the slot centre is the same midpoint the insertion index is chosen against,
// so the side can flip without the insertion index changing
export function boundarySideForSlot(
  slotCenter: number | null,
  anchorY: number,
): BoundarySide | null {
  if (slotCenter === null) {
    return null;
  }
  if (anchorY < slotCenter) {
    return 'above';
  }
  if (anchorY > slotCenter) {
    return 'below';
  }
  return null;
}

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

export type PointerPreview = {
  items: Record<string, string[]>;
  slots: TaskSlot[];
  index: number;
  /** Centre of the placeholder slot the dragged card would occupy. */
  slotCenter: number | null;
};

/**
 * Returns the original `items` when the preview matches it exactly, so callers
 * can skip re-rendering.
 */
export function previewAtPointerDetailed(
  items: Record<string, string[]>,
  layout: DragLayout,
  destination: string,
  dragId: string,
  anchorY: number,
): PointerPreview {
  const withoutDrag: Record<string, string[]> = {};
  for (const [group, ids] of Object.entries(items)) {
    withoutDrag[group] = ids.filter((id) => id !== dragId);
  }

  if (!(destination in items)) {
    return { items, slots: [], index: -1, slotCenter: null };
  }

  const groupLayout = layout.groups[destination];
  if (groupLayout === undefined) {
    return { items, slots: [], index: -1, slotCenter: null };
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

  const index = insertionIndexForPointer(slots, anchorY, layout.draggedHeight, groupLayout.gap);
  const nextDestination = [...withoutDrag[destination]];
  nextDestination.splice(index, 0, dragId);

  const next: Record<string, string[]> = { ...withoutDrag, [destination]: nextDestination };

  const currentDestination = items[destination];
  const unchanged =
    currentDestination.length === nextDestination.length &&
    currentDestination.every((id, position) => id === nextDestination[position]);

  // the slot centre is the same midpoint the insertion index is chosen against,
  // so the side can flip without the insertion index changing
  const slotTop =
    index < slots.length
      ? slots[index].top
      : (slots[slots.length - 1]?.bottom ?? 0) + groupLayout.gap;
  const slotCenter = slotTop + layout.draggedHeight / 2;

  return { items: unchanged ? items : next, slots, index, slotCenter };
}

export function previewAtPointer(
  items: Record<string, string[]>,
  layout: DragLayout,
  destination: string,
  dragId: string,
  anchorY: number,
): Record<string, string[]> {
  return previewAtPointerDetailed(items, layout, destination, dragId, anchorY).items;
}

export function sectionTasks(project: ProjectSnapshot, sectionId: string | null): Task[] {
  return project.tasks
    .filter((task) => task.section_id === sectionId)
    .sort((left, right) => compareTasks('manual', left, right));
}

export type SoloHome = { top: number; height: number };

/**
 * The original slot centre of a dragged card when it is the only card in its
 * priority band. That band vanishes from the preview once the card moves
 * elsewhere, so this point is the only place the card can be dropped back into
 * its priority.
 */
export function soloPriorityHome(
  group: BoardGroup,
  taskId: string,
  layout: DragLayout,
): SoloHome | null {
  const task = group.tasks.find((candidate) => candidate.id === taskId);
  if (task === undefined) {
    return null;
  }
  const samePriority = group.tasks.filter((candidate) => candidate.priority === task.priority);
  if (samePriority.length !== 1) {
    return null;
  }
  const slot = layout.groups[groupKey(group)]?.slots.find((candidate) => candidate.id === taskId);
  return slot === undefined
    ? null
    : { top: slot.top, height: Math.min(slot.bottom - slot.top, 48) };
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

// a boundary slot belongs to both neighbouring bands. The pointer's side picks
// one; without it a same-slot drop keeps the current priority so "bottom of my
// band" stays reachable, and the upper band is the fallback.
export function dragPriority(
  project: ProjectSnapshot,
  mode: TaskSortingMode,
  baseItems: Record<string, string[]>,
  finalItems: Record<string, string[]>,
  taskId: string,
  side: BoundarySide | null = null,
): Priority | null {
  const task = project.tasks.find((candidate) => candidate.id === taskId);
  if (mode !== 'priority' || task === undefined) {
    return null;
  }

  const originKey = groupContaining(baseItems, taskId);
  const originIndex = originKey === null ? -1 : baseItems[originKey].indexOf(taskId);
  const destinationKey = groupContaining(finalItems, taskId);
  if (destinationKey === null) {
    return task.priority;
  }

  const destinationIds = finalItems[destinationKey] ?? [];
  const destinationIndex = destinationIds.indexOf(taskId);
  const sameSlot = originKey === destinationKey && originIndex === destinationIndex;

  const abovePriority =
    destinationIndex > 0
      ? (project.tasks.find((candidate) => candidate.id === destinationIds[destinationIndex - 1])
          ?.priority ?? null)
      : null;
  const belowPriority =
    destinationIndex === -1
      ? null
      : (project.tasks.find((candidate) => candidate.id === destinationIds[destinationIndex + 1])
          ?.priority ?? null);

  if (abovePriority !== null && belowPriority !== null && abovePriority !== belowPriority) {
    // the pointer side wins even when the card already belongs to one of the
    // bands, so both "bottom of this band" and "top of the next" are reachable
    if (side === 'above') {
      return abovePriority;
    }
    if (side === 'below') {
      return belowPriority;
    }
    if (sameSlot) {
      return task.priority;
    }
    if (task.priority === abovePriority) {
      return abovePriority;
    }
    if (task.priority === belowPriority) {
      return belowPriority;
    }
    return abovePriority;
  }

  if (abovePriority !== null && belowPriority !== null) {
    return abovePriority;
  }
  if (abovePriority !== null) {
    return abovePriority;
  }
  if (belowPriority !== null) {
    return belowPriority;
  }
  return task.priority;
}

// the priority view compares priority before position, so only same-priority
// tasks define the insertion point; other priorities must not shift the key
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
