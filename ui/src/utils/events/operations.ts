import { sameDue } from '@/contract/due';
import { nowIso } from '@/utils/date';
import type { Event } from '@/generated/Event';
import type { Interval } from '@/generated/Interval';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { newId } from '@/utils/uuid';

export type EventFieldPatch = {
  title: string;
  description: string;
  interval: Interval;
};

export type CreatedEvent = { id: string; operation: Operation };

export function createEventOperation(
  project: ProjectSnapshot,
  patch: EventFieldPatch,
): CreatedEvent | null {
  const title = patch.title.trim();
  if (title.length === 0) {
    return null;
  }

  const description = patch.description.trim();
  const id = newId();
  const created_at = nowIso();

  return {
    id,
    operation: {
      operation_id: newId(),
      operation_version: 1,
      kind: 'create_event',
      id,
      project_id: project.id,
      title,
      description: description.length === 0 ? null : description,
      interval: patch.interval,
      created_at,
      updated_at: created_at,
    },
  };
}

export function updateEventOperation(event: Event, patch: EventFieldPatch): Operation | null {
  const title = patch.title.trim();
  if (title.length === 0) {
    return null;
  }

  const description = patch.description.trim();
  const nextDescription = description.length === 0 ? null : description;

  const titleChanged = title !== event.title;
  const descriptionChanged = nextDescription !== event.description;
  const intervalChanged = !sameInterval(patch.interval, event.interval);

  if (!titleChanged && !descriptionChanged && !intervalChanged) {
    return null;
  }

  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'update_event',
    id: event.id,
    title: titleChanged ? title : null,
    description: descriptionChanged ? nextDescription : undefined,
    interval: intervalChanged ? patch.interval : null,
    updated_at: nowIso(),
  };
}

export function deleteEventOperation(event: Event): Operation {
  return {
    operation_id: newId(),
    operation_version: 1,
    kind: 'delete_event',
    id: event.id,
  };
}

function sameInterval(left: Interval, right: Interval): boolean {
  return sameDue(left.start, right.start) && sameDue(left.end, right.end);
}
