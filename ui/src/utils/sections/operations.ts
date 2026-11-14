import { keyBetween } from '@/contract/positions';
import type { Color } from '@/generated/Color';
import { newId } from '@/utils/uuid';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';

export function createSectionOperation(project: ProjectSnapshot, name: string): Operation | null {
  const trimmed = name.trim();
  if (trimmed.length === 0) {
    return null;
  }

  const lastPosition = project.sections.reduce<string | null>(
    (max, section) => (max === null || section.position > max ? section.position : max),
    null,
  );
  const created_at = new Date().toISOString();

  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'create_section',
    id: newId(),
    project_id: project.id,
    name: trimmed,
    position: keyBetween(lastPosition, null),
    color: null,
    created_at,
    updated_at: created_at,
  };
}

export function moveSectionOperation(sectionId: string, position: string): Operation {
  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'move_section',
    id: sectionId,
    position,
    updated_at: new Date().toISOString(),
  };
}

export function deleteSectionOperation(sectionId: string): Operation {
  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'delete_section',
    id: sectionId,
  };
}

export function updateSectionOperation(sectionId: string, name: string): Operation | null {
  const trimmed = name.trim();
  if (trimmed.length === 0) {
    return null;
  }

  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'update_section',
    id: sectionId,
    name: trimmed,
    color: null,
    updated_at: new Date().toISOString(),
  };
}

export function setSectionColorOperation(sectionId: string, color: Color | null): Operation {
  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'update_section',
    id: sectionId,
    name: null,
    color: color === null ? 'clear' : { set: color },
    updated_at: new Date().toISOString(),
  };
}
