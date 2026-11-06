import type { Operation } from '@/generated/Operation';

export function updateSectionOperation(sectionId: string, name: string): Operation | null {
  const trimmed = name.trim();
  if (trimmed.length === 0) {
    return null;
  }

  return {
    operation_id: crypto.randomUUID(),
    operation_version: 1,
    kind: 'update_section',
    id: sectionId,
    name: trimmed,
    color: null,
  };
}
