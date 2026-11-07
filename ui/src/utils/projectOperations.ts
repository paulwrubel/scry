import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { newId } from '@/utils/uuid';
import type { TaskSortingMode } from '@/generated/TaskSortingMode';

export type ProjectFieldPatch = {
  name: string;
  description: string;
  task_sorting_mode: TaskSortingMode;
  show_priority: boolean;
};

export function updateProjectOperation(
  project: ProjectSnapshot,
  patch: ProjectFieldPatch,
): Operation | null {
  const name = patch.name.trim();
  if (name.length === 0) {
    return null;
  }

  const description = patch.description.trim();
  const nextDescription = description.length === 0 ? null : description;

  const nameChanged = name !== project.name;
  const descriptionChanged = nextDescription !== project.description;
  const sortingChanged = patch.task_sorting_mode !== project.task_sorting_mode;
  const priorityChanged = patch.show_priority !== project.show_priority;

  if (!nameChanged && !descriptionChanged && !sortingChanged && !priorityChanged) {
    return null;
  }

  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'update_project',
    id: project.id,
    name: nameChanged ? name : null,
    description: descriptionChanged ? nextDescription : undefined,
    task_sorting_mode: sortingChanged ? patch.task_sorting_mode : null,
    show_priority: priorityChanged ? patch.show_priority : null,
  };
}
