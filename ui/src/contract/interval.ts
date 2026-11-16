import { DateTime } from 'luxon';
import type { Interval } from '@/generated/Interval';
import type { When } from '@/generated/When';
import { DEVICE_ZONE, dayPart } from './due';

/// Labels stay English so the web UI reads the same as the CLI's formatter.
const LABEL_LOCALE = 'en-US';

/// The default length of an event that names only a start, mirroring
/// `DEFAULT_EVENT_MINUTES` in `scry-core`.
const DEFAULT_EVENT_MINUTES = 60;

/// The two shapes the editor can produce: a whole day, or a local time in the
/// device zone, which is the only zone the browser exposes.
type InputWhen = Extract<When, { kind: 'date' }> | Extract<When, { kind: 'zoned' }>;

export type IntervalInputs = {
  startDate: string;
  startTime: string;
  endDate: string;
  endTime: string;
};

export type IntervalResult =
  { interval: Interval; error: null } | { interval: null; error: string };

/// `start` is inclusive and `end` is exclusive, so the last day a date interval
/// covers is the day before its end.
export function formatInterval(
  interval: Interval,
  now: DateTime = DateTime.now(),
  zone: string = DEVICE_ZONE,
): string {
  const today = now.setZone(zone);
  const startClock = displayedClock(interval.start, zone);

  if (startClock === null) {
    const startDay = instantOf(interval.start, zone);
    const end = inclusiveEnd(interval, startDay, zone);
    const label = dayPart(startDay, today);
    return end.hasSame(startDay, 'day') ? label : `${label} – ${absoluteDate(end, today)}`;
  }

  const startDay = startClock.startOf('day');
  const endClock = displayedClock(interval.end, zone) ?? startClock;
  const endDay = endClock.startOf('day');
  const label = `${dayPart(startDay, today)} ${formatTimeOfDay(startClock)}`;
  const suffix = endDay.hasSame(startDay, 'day')
    ? formatTimeOfDay(endClock)
    : `${dayPart(endDay, today)} ${formatTimeOfDay(endClock)}`;

  return `${label} – ${suffix}${whenTag(interval.start, zone)}`;
}

/// True when the interval covers whole calendar days rather than a time of day.
export function isAllDay(interval: Interval): boolean {
  return interval.start.kind === 'date';
}

/// An interval from the editor's four fields. An empty start time makes an
/// all-day event; leaving both end fields empty applies the default length,
/// which is a whole day for a date and one hour for a time.
export function intervalFromInputs(inputs: IntervalInputs): IntervalResult {
  if (inputs.startDate.length === 0) {
    return { interval: null, error: 'Choose a start date.' };
  }

  const start = civilValue(inputs.startDate, inputs.startTime);
  if (start === null) {
    return { interval: null, error: 'That start date and time is not valid.' };
  }

  if (inputs.endDate.length === 0 && inputs.endTime.length === 0) {
    return { interval: defaultInterval(start), error: null };
  }

  if (start.kind === 'zoned' && inputs.endTime.length === 0) {
    return {
      interval: null,
      error: 'Choose an end time, or leave both end fields empty for the default.',
    };
  }

  // a timed end on its own means the same day, so only the end time is required
  const rawEnd = civilValue(
    inputs.endDate.length === 0 ? inputs.startDate : inputs.endDate,
    inputs.endTime,
  );
  if (rawEnd === null) {
    return { interval: null, error: 'That end date and time is not valid.' };
  }

  // the stored end is exclusive, so the day the user picked is covered through
  // its end and the interval ends at the next midnight
  const end: InputWhen =
    rawEnd.kind === 'date'
      ? { kind: 'date', date: formatDay(civilDate(rawEnd.date).plus({ days: 1 })) }
      : rawEnd;

  if (instantOf(end, DEVICE_ZONE) <= instantOf(start, DEVICE_ZONE)) {
    return { interval: null, error: 'The end must be after the start.' };
  }

  return { interval: { start, end }, error: null };
}

/// The editor fields for an existing interval, with a date end turned back into
/// the inclusive last day the calendar shows.
export function intervalToInputs(interval: Interval): IntervalInputs {
  const start = dueInputs(interval.start);
  if (interval.end.kind !== 'date') {
    const end = dueInputs(interval.end);
    return { startDate: start.date, startTime: start.time, endDate: end.date, endTime: end.time };
  }

  return {
    startDate: start.date,
    startTime: start.time,
    endDate: formatDay(civilDate(interval.end.date).minus({ days: 1 })),
    endTime: '',
  };
}

/// The end fields a new event shows for these start values: the same day for a
/// date, one hour later for a time. An event always has an end, so the editor
/// shows the one it would default to instead of leaving the fields blank.
export function defaultEndInputs(
  startDate: string,
  startTime: string,
): { endDate: string; endTime: string } {
  const start = civilValue(startDate, startTime);
  if (start === null) {
    return { endDate: '', endTime: '' };
  }
  const inputs = intervalToInputs(defaultInterval(start));
  return { endDate: inputs.endDate, endTime: inputs.endTime };
}

/// The moment a `When` names, read in the given zone when it carries no zone of
/// its own.
export function instantOf(when: When, zone: string): DateTime {
  switch (when.kind) {
    case 'date':
      return DateTime.fromISO(when.date, { zone }).startOf('day');
    case 'floating':
      return DateTime.fromISO(when.datetime, { zone });
    case 'instant':
      return DateTime.fromISO(when.timestamp, { setZone: true });
    case 'zoned':
      return DateTime.fromISO(when.datetime, { zone: when.zone });
  }
}

/// The clock time the calendar and interval labels show.
export function formatTimeOfDay(value: DateTime): string {
  return value.toFormat('h:mm a', { locale: LABEL_LOCALE });
}

/// The moment whose clock time a value displays as: its own civil time when it
/// carries one, and the viewer's conversion only for an instant. All-day dates
/// have no clock, so they return null.
export function displayedClock(when: When, zone: string): DateTime | null {
  if (when.kind === 'date') {
    return null;
  }
  // a zoned value keeps its own zone so the label shows the time the user typed
  return when.kind === 'zoned'
    ? DateTime.fromISO(when.datetime, { zone: when.zone })
    : instantOf(when, zone).setZone(zone);
}

function inclusiveEnd(interval: Interval, start: DateTime, zone: string): DateTime {
  if (interval.end.kind !== 'date') {
    return start;
  }
  return DateTime.fromISO(interval.end.date, { zone }).startOf('day').minus({ days: 1 });
}

function absoluteDate(value: DateTime, today: DateTime): string {
  return value.year === today.year
    ? value.toFormat('LLL d', { locale: LABEL_LOCALE })
    : value.toFormat('LLL d, yyyy', { locale: LABEL_LOCALE });
}

function whenTag(when: When, zone: string): string {
  if (when.kind === 'zoned') {
    return when.zone === zone ? '' : ` (${when.zone})`;
  }
  return when.kind === 'floating' ? ' (floating)' : '';
}

function civilValue(date: string, time: string): InputWhen | null {
  if (time.length === 0) {
    const day = DateTime.fromISO(date);
    return day.isValid ? { kind: 'date', date: formatDay(day) } : null;
  }

  const value = DateTime.fromISO(`${date}T${time}`, { zone: DEVICE_ZONE });
  if (!value.isValid) {
    return null;
  }
  return { kind: 'zoned', datetime: value.toFormat("yyyy-LL-dd'T'HH:mm:ss"), zone: DEVICE_ZONE };
}

function dueInputs(when: When): { date: string; time: string } {
  switch (when.kind) {
    case 'date':
      return { date: when.date, time: '' };
    case 'floating':
      return civilInputs(DateTime.fromISO(when.datetime));
    case 'instant':
      return civilInputs(DateTime.fromISO(when.timestamp, { setZone: true }).setZone(DEVICE_ZONE));
    case 'zoned':
      return civilInputs(DateTime.fromISO(when.datetime, { zone: when.zone }));
  }
}

/// One hour after a time, or the whole next day for a date, matching the
/// server's `Interval::from_start`.
function defaultInterval(start: InputWhen): Interval {
  if (start.kind === 'date') {
    return {
      start,
      end: { kind: 'date', date: formatDay(civilDate(start.date).plus({ days: 1 })) },
    };
  }

  return {
    start,
    end: {
      kind: 'zoned',
      datetime: DateTime.fromISO(start.datetime, { zone: start.zone })
        .plus({ minutes: DEFAULT_EVENT_MINUTES })
        .toFormat("yyyy-LL-dd'T'HH:mm:ss"),
      zone: start.zone,
    },
  };
}

function civilDate(value: string): DateTime {
  return DateTime.fromISO(value);
}

function civilInputs(value: DateTime): { date: string; time: string } {
  return { date: formatDay(value), time: value.toFormat('HH:mm') };
}

function formatDay(value: DateTime): string {
  return value.isValid ? (value.toISODate() ?? '') : '';
}
