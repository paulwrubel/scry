import { keyBetween } from '@/contract/positions';
import { compareTasks } from '@/contract/task-order';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import type { TaskSortingMode } from '@/generated/TaskSortingMode';
import type { TaskFilter } from '@/utils/taskFilter';
import { matchesTaskFilter } from '@/utils/taskFilter';

export type BoardGroup = {
  section: Section | null;
  tasks: Task[];
};

export const NO_SECTION_GROUP = 'no-section';

export function groupKey(sectionId: string | null): string {
  return sectionId ?? NO_SECTION_GROUP;
}

export function sectionIdFromGroup(key: string): string | null {
  return key === NO_SECTION_GROUP ? null : key;
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
    tasks: orderTasks(
      visible.filter((task) => task.section_id === section.id),
      project.task_sorting_mode,
    ),
  }));

  const unsectioned = orderTasks(
    visible.filter((task) => task.section_id === null),
    project.task_sorting_mode,
  );

  return unsectioned.length === 0 ? groups : [{ section: null, tasks: unsectioned }, ...groups];
}

export function itemsFromGroups(groups: BoardGroup[]): Record<string, string[]> {
  return Object.fromEntries(
    groups.map((group) => [
      groupKey(group.section?.id ?? null),
      group.tasks.map((task) => task.id),
    ]),
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
