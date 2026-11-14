import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { newId } from '@/utils/uuid';

export type ProjectFieldPatch = {
  name: string;
  description: string;
};

export type ProjectDraft = {
  name: string;
  description: string;
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

  if (!nameChanged && !descriptionChanged) {
    return null;
  }

  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'update_project',
    id: project.id,
    name: nameChanged ? name : null,
    description: descriptionChanged ? nextDescription : undefined,
    updated_at: new Date().toISOString(),
  };
}

export function createProjectOperation(draft: ProjectDraft): Operation {
  const name = draft.name.trim();
  const description = draft.description.trim();
  const created_at = new Date().toISOString();

  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'create_project',
    id: newId(),
    name,
    description: description.length === 0 ? null : description,
    created_at,
    updated_at: created_at,
  };
}

export function deleteProjectOperation(project: ProjectSnapshot): Operation {
  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'delete_project',
    id: project.id,
  };
}
