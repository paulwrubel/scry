import { keyBetween } from '@/contract/positions';
import { compareTasks } from '@/contract/task-order';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import type { TaskSortingMode } from '@/generated/TaskSortingMode';
import type { TaskState } from '@/generated/TaskState';
import type { TaskFilter } from '@/utils/taskFilter';
import { matchesTaskFilter } from '@/utils/taskFilter';

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

export function buildBoard(project: ProjectSnapshot, filter: TaskFilter): BoardGroup[] {
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
      project.task_sorting_mode,
    ),
  }));

  const openNoSection = orderTasks(
    visible.filter((task) => task.section_id === null && task.state === 'open'),
    project.task_sorting_mode,
  );

  const closedNoSection = orderTasks(
    visible.filter((task) => task.section_id === null && task.state === 'closed'),
    project.task_sorting_mode,
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

function orderTasks(tasks: Task[], mode: TaskSortingMode): Task[] {
  return tasks.sort((left, right) => compareTasks(mode, left, right));
}
