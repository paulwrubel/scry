import { keyBetween } from '@/contract/positions';
import { compareTasks } from '@/contract/task-order';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Section } from '@/generated/Section';
import type { Task } from '@/generated/Task';
import type { TaskSortingMode } from '@/generated/TaskSortingMode';

export type BoardGroup = {
  section: Section | null;
  tasks: Task[];
};

export function buildBoard(project: ProjectSnapshot, showClosed: boolean): BoardGroup[] {
  const visible = project.tasks.filter((task) => showClosed || task.state === 'open');

  const groups: BoardGroup[] = project.sections.map((section) => ({
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

  if (unsectioned.length > 0) {
    groups.push({ section: null, tasks: unsectioned });
  }

  return groups;
}

export function nextTaskPosition(tasks: readonly Task[], sectionId: string): string {
  const last = tasks
    .filter((task) => task.section_id === sectionId)
    .reduce<string | null>(
      (max, task) => (max === null || task.position > max ? task.position : max),
      null,
    );

  return keyBetween(last, null);
}

function orderTasks(tasks: Task[], mode: TaskSortingMode): Task[] {
  return tasks.sort((left, right) => compareTasks(mode, left, right));
}
