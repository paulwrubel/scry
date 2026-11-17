import { sameDue } from '@/contract/due';
import { nowIso } from '@/utils/date';
import type { Event } from '@/generated/Event';
import type { Interval } from '@/generated/Interval';
import type { Operation } from '@/generated/Operation';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { RRule } from '@/generated/RRule';
import type { When } from '@/generated/When';
import { newId } from '@/utils/uuid';

export type EventFieldPatch = {
  title: string;
  description: string;
  interval: Interval;
  recurrence: RRule | null;
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
      timestamp: nowIso(),
      kind: 'create_event',
      id,
      project_id: project.id,
      title,
      description: description.length === 0 ? null : description,
      interval: patch.interval,
      recurrence: patch.recurrence,
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
  const recurrenceChanged = !sameRecurrence(patch.recurrence, event.recurrence ?? null);

  if (!titleChanged && !descriptionChanged && !intervalChanged && !recurrenceChanged) {
    return null;
  }

  return {
    operation_id: newId(),
    operation_version: 1,
    timestamp: nowIso(),
    kind: 'update_event',
    id: event.id,
    title: titleChanged ? title : null,
    description: descriptionChanged ? nextDescription : undefined,
    interval: intervalChanged ? patch.interval : null,
    recurrence: recurrenceChanged ? patch.recurrence : undefined,
    updated_at: nowIso(),
  };
}

export function deleteEventOperation(event: Event): Operation {
  return {
    operation_id: newId(),
    operation_version: 1,
    timestamp: nowIso(),
    kind: 'delete_event',
    id: event.id,
  };
}

export function deleteOccurrenceOperation(event: Event, occurrence: When): Operation | null {
  const recurrence = event.recurrence;
  if (recurrence === null || recurrence === undefined) {
    return null;
  }

  const exdates = recurrence.exdates ?? [];
  if (exdates.some((exdate) => sameDue(exdate, occurrence))) {
    return null;
  }

  return updateEventOperation(event, {
    title: event.title,
    description: event.description ?? '',
    interval: event.interval,
    recurrence: { ...recurrence, exdates: [...exdates, occurrence] },
  });
}

function sameInterval(left: Interval, right: Interval): boolean {
  return sameDue(left.start, right.start) && sameDue(left.end, right.end);
}

function sameRecurrence(left: RRule | null, right: RRule | null): boolean {
  if (left === null || right === null) {
    return left === right;
  }

  if (left.text !== right.text) {
    return false;
  }

  const leftExdates = left.exdates ?? [];
  const rightExdates = right.exdates ?? [];
  if (leftExdates.length !== rightExdates.length) {
    return false;
  }

  return leftExdates.every((exdate, index) => sameDue(exdate, rightExdates[index]));
}
