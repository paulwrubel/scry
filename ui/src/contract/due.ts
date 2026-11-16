import { DateTime } from 'luxon';
import type { Task } from '@/generated/Task';
import type { When } from '@/generated/When';

/// Labels stay English so the web UI reads the same as the CLI's formatter.
const LABEL_LOCALE = 'en-US';
export const DEVICE_ZONE: string = DateTime.local().zoneName;

export function deadline(due: When, zone: string = DEVICE_ZONE): DateTime {
  switch (due.kind) {
    case 'date':
      // an all-day value covers its whole local day, so it is late at the next local midnight
      return DateTime.fromISO(due.date, { zone }).plus({ days: 1 }).startOf('day');
    case 'floating':
      return DateTime.fromISO(due.datetime, { zone });
    case 'instant':
      return DateTime.fromISO(due.timestamp, { setZone: true });
    case 'zoned':
      return DateTime.fromISO(due.datetime, { zone: due.zone });
  }
}

export function isOverdue(
  task: Pick<Task, 'due' | 'state'>,
  now: DateTime = DateTime.now(),
  zone: string = DEVICE_ZONE,
): boolean {
  if (task.due === null || task.state === 'closed') {
    return false;
  }
  const expiry = deadline(task.due, zone);
  return expiry.isValid && expiry.diff(now, 'milliseconds').milliseconds <= 0;
}

export function formatDue(
  due: When,
  now: DateTime = DateTime.now(),
  zone: string = DEVICE_ZONE,
): string {
  const today = now.setZone(zone);
  const value = dueDateTime(due, zone);

  let label = dayPart(value, today);
  if (due.kind !== 'date') {
    label += ` ${value.toFormat('h:mm a', { locale: LABEL_LOCALE })}`;
  }

  if (due.kind === 'zoned') {
    if (due.zone !== zone) {
      label += ` (${due.zone})`;
    }
  } else if (due.kind === 'floating') {
    label += ' (floating)';
  }

  return label;
}

function dueDateTime(due: When, zone: string): DateTime {
  switch (due.kind) {
    case 'date':
      return DateTime.fromISO(due.date, { zone });
    case 'floating':
      return DateTime.fromISO(due.datetime, { zone });
    case 'instant':
      return DateTime.fromISO(due.timestamp, { setZone: true }).setZone(zone);
    case 'zoned':
      return DateTime.fromISO(due.datetime, { zone: due.zone });
  }
}

/// Calendar days apart, not elapsed time, so a value in a far zone keeps its own date.
export function dayPart(value: DateTime, today: DateTime): string {
  const startOfToday = DateTime.fromObject(
    { year: today.year, month: today.month, day: today.day },
    { zone: value.zone },
  );
  const days = Math.round(value.startOf('day').diff(startOfToday, 'days').days);

  switch (days) {
    case -1:
      return 'yesterday';
    case 0:
      return 'today';
    case 1:
      return 'tomorrow';
    default:
      if (days >= 2 && days <= 6) {
        return `in ${days}d`;
      }
      return value.year === today.year
        ? value.toFormat('LLL d', { locale: LABEL_LOCALE })
        : value.toFormat('LLL d, yyyy', { locale: LABEL_LOCALE });
  }
}

export function dueFromInputs(date: string, time: string): When | null {
  if (date.length === 0) {
    return null;
  }
  if (time.length === 0) {
    const day = DateTime.fromISO(date);
    return day.isValid ? { kind: 'date', date: day.toISODate() ?? date } : null;
  }
  const value = DateTime.fromISO(`${date}T${time}`, { zone: DEVICE_ZONE });
  if (!value.isValid) {
    return null;
  }
  return {
    kind: 'zoned',
    datetime: value.toFormat("yyyy-LL-dd'T'HH:mm:ss"),
    zone: DEVICE_ZONE,
  };
}

export function dueToInputs(due: When | null): { date: string; time: string } {
  if (due === null) {
    return { date: '', time: '' };
  }
  switch (due.kind) {
    case 'date':
      return { date: DateTime.fromISO(due.date).toISODate() ?? '', time: '' };
    case 'floating':
    case 'zoned':
      return civilInputs(DateTime.fromISO(due.datetime));
    case 'instant':
      return civilInputs(DateTime.fromISO(due.timestamp, { setZone: true }).setZone(DEVICE_ZONE));
  }
}

function civilInputs(value: DateTime): { date: string; time: string } {
  return { date: value.toISODate() ?? '', time: value.toFormat('HH:mm') };
}

export function sameDue(left: When | null, right: When | null): boolean {
  if (left === null || right === null) {
    return left === right;
  }
  switch (left.kind) {
    case 'date':
      return right.kind === 'date' && left.date === right.date;
    case 'floating':
      return right.kind === 'floating' && left.datetime === right.datetime;
    case 'instant':
      return right.kind === 'instant' && left.timestamp === right.timestamp;
    case 'zoned':
      return right.kind === 'zoned' && left.datetime === right.datetime && left.zone === right.zone;
  }
}
