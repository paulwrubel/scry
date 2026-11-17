import { DateTime } from 'luxon';
import { instantOf, isAllDay, displayedClock } from '@/contract/interval';
import { expandRecurrence } from '@/contract/recur';
import type { Event } from '@/generated/Event';
import type { Interval } from '@/generated/Interval';
import type { When } from '@/generated/When';

export const MONTH_PARAM = 'month';
export const VIEW_PARAM = 'view';
export const WEEK_PARAM = 'week';

export type Month = { year: number; month: number };

export type CalendarView = 'month' | 'week';

export type CalendarEvent = Event & {
  /// The occurrence's own start, present only for an expanded series.
  occurrenceStart?: When;
};

export type EventPlacement = {
  event: CalendarEvent;
  /// Inclusive column range within the week, 0–6.
  startIndex: number;
  endIndex: number;
  continuesBefore: boolean;
  continuesAfter: boolean;
  /// The start clock time, only on the segment where the event begins.
  startTime: DateTime | null;
  /// The end clock time, only on the segment where the event ends.
  endTime: DateTime | null;
};

/// One row of bars in a week. Bars in the same lane never overlap.
export type WeekLane = EventPlacement[];

export type FittedLanes = {
  visible: WeekLane[];
  /// Hidden bars per weekday column, 0–6. Zero everywhere when nothing is hidden.
  hiddenByDay: number[];
};

/// Keeps only the lanes that fit in `capacity` rows, reserving the last row for
/// the overflow count. A null capacity means the height is not known yet, so
/// nothing is hidden.
export function fitLanes(lanes: readonly WeekLane[], capacity: number | null): FittedLanes {
  const rows = capacity === null ? lanes.length : Math.max(1, capacity);
  if (lanes.length <= rows) {
    return { visible: [...lanes], hiddenByDay: hiddenCount(7) };
  }

  const hiddenByDay = hiddenCount(7);
  for (const placement of lanes.slice(rows - 1).flat()) {
    for (let day = placement.startIndex; day <= placement.endIndex; day += 1) {
      hiddenByDay[day] += 1;
    }
  }

  return { visible: lanes.slice(0, rows - 1), hiddenByDay };
}

function hiddenCount(columns: number): number[] {
  return Array.from({ length: columns }, () => 0);
}

/// The events that cover `day`, all-day ones first and then by start time.
export function eventsOnDay(
  events: readonly CalendarEvent[],
  day: DateTime,
  zone: string,
): CalendarEvent[] {
  const target = day.setZone(zone).startOf('day');

  return events
    .filter((event) => {
      const span = eventSpan(event, zone);
      return target >= span.first && target <= span.last;
    })
    .sort((left, right) => compareEventStarts(left, right, zone));
}

function compareEventStarts(left: Event, right: Event, zone: string): number {
  const leftStart = displayedClock(left.interval.start, zone);
  const rightStart = displayedClock(right.interval.start, zone);

  if (leftStart === null || rightStart === null) {
    if (leftStart !== rightStart) {
      return leftStart === null ? -1 : 1;
    }
  } else if (leftStart.toMillis() !== rightStart.toMillis()) {
    return leftStart.toMillis() - rightStart.toMillis();
  }

  return left.title.localeCompare(right.title);
}

/// Expand every recurring event over the window and leave plain events alone,
/// sorted by their actual start. An invalid stored rule falls back to the
/// series once so one bad event cannot blank the whole calendar.
export function expandEvents(
  events: readonly Event[],
  from: DateTime,
  to: DateTime,
  zone: string,
): CalendarEvent[] {
  const expanded: CalendarEvent[] = [];

  for (const event of events) {
    const rrule = event.recurrence;
    if (rrule === null || rrule === undefined) {
      expanded.push(event);
      continue;
    }

    let intervals: Interval[];
    try {
      intervals = expandRecurrence(event.interval, rrule, from, to, zone);
    } catch {
      intervals = [event.interval];
    }

    for (const interval of intervals) {
      expanded.push({ ...event, interval, occurrenceStart: interval.start });
    }
  }

  expanded.sort((left, right) => {
    const difference =
      instantOf(left.interval.start, zone).toMillis() -
      instantOf(right.interval.start, zone).toMillis();
    return difference === 0 ? left.id.localeCompare(right.id) : difference;
  });

  return expanded;
}

/// A stable React key for a series row or one expanded occurrence.
export function eventKey(event: CalendarEvent): string {
  return event.occurrenceStart === undefined
    ? event.id
    : `${event.id}:${JSON.stringify(event.occurrenceStart)}`;
}

/// The event detail path, carrying the occurrence start when this is a series occurrence.
export function eventPath(projectId: string, event: CalendarEvent, query: string): string {
  const params = new URLSearchParams(query.startsWith('?') ? query.slice(1) : query);

  if (event.occurrenceStart !== undefined) {
    params.set('occurrence', JSON.stringify(event.occurrenceStart));
  }

  const suffix = params.size === 0 ? '' : `?${params.toString()}`;
  return `/projects/${projectId}/events/${event.id}${suffix}`;
}

/// Read the encoded occurrence start from an event detail link. Returns null
/// for a missing or malformed value rather than trusting arbitrary JSON.
export function parseOccurrence(value: string | null): When | null {
  if (value === null) {
    return null;
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(value);
  } catch {
    return null;
  }

  return isWhen(parsed) ? parsed : null;
}

function isWhen(value: unknown): value is When {
  if (typeof value !== 'object' || value === null || !('kind' in value)) {
    return false;
  }

  const kind = value.kind;
  if (kind === 'date') {
    return 'date' in value && typeof value.date === 'string';
  }
  if (kind === 'floating') {
    return 'datetime' in value && typeof value.datetime === 'string';
  }
  if (kind === 'instant') {
    return 'timestamp' in value && typeof value.timestamp === 'string';
  }
  if (kind === 'zoned') {
    return (
      'datetime' in value &&
      typeof value.datetime === 'string' &&
      'zone' in value &&
      typeof value.zone === 'string'
    );
  }
  return false;
}

/// The visible month, read from the URL and falling back to the current one so
/// a default view keeps a clean link.
export function parseMonth(params: URLSearchParams, now: DateTime = DateTime.now()): Month {
  const parsed = DateTime.fromFormat(params.get(MONTH_PARAM) ?? '', 'yyyy-LL');
  return parsed.isValid
    ? { year: parsed.year, month: parsed.month }
    : { year: now.year, month: now.month };
}

export function applyMonth(params: URLSearchParams, month: Month): URLSearchParams {
  const next = new URLSearchParams(params);
  next.set(MONTH_PARAM, monthKey(month));
  return next;
}

/// Month is the default, so a missing parameter and any value we no longer
/// recognise both fall back to it.
export function parseCalendarView(params: URLSearchParams): CalendarView {
  return params.get(VIEW_PARAM) === 'week' ? 'week' : 'month';
}

export function applyCalendarView(params: URLSearchParams, view: CalendarView): URLSearchParams {
  const next = new URLSearchParams(params);
  next.set(VIEW_PARAM, view);
  return next;
}

export function monthKey(month: Month): string {
  return DateTime.fromObject({ year: month.year, month: month.month, day: 1 }).toFormat('yyyy-LL');
}

export function shiftMonth(month: Month, delta: number): Month {
  const shifted = DateTime.fromObject({ year: month.year, month: month.month, day: 1 }).plus({
    months: delta,
  });
  return { year: shifted.year, month: shifted.month };
}

export function monthLabel(month: Month): string {
  return DateTime.fromObject({ year: month.year, month: month.month, day: 1 }).toFormat(
    'LLLL yyyy',
  );
}

/// Whole weeks from the Sunday on or before the first of the month to the
/// Saturday on or after its last day, so every week row is full.
export function monthGrid(month: Month, zone: string): DateTime[][] {
  const first = DateTime.fromObject(
    { year: month.year, month: month.month, day: 1 },
    { zone },
  ).startOf('day');
  const gridStart = first.minus({ days: first.weekday % 7 });
  const last = first.endOf('month').startOf('day');
  const gridEnd = last.plus({ days: 6 - (last.weekday % 7) });

  const weeks: DateTime[][] = [];
  let cursor = gridStart;
  while (cursor <= gridEnd) {
    const week: DateTime[] = [];
    for (let offset = 0; offset < 7; offset += 1) {
      week.push(cursor.plus({ days: offset }));
    }
    weeks.push(week);
    cursor = cursor.plus({ days: 7 });
  }
  return weeks;
}

/// The Sunday on or before `day`, at its start. Luxon counts Monday as 1 and
/// Sunday as 7, so the modulo lands Sunday on zero days back.
export function weekStart(day: DateTime): DateTime {
  return day.minus({ days: day.weekday % 7 }).startOf('day');
}

/// The week containing the `week` parameter, or the current week when it is
/// missing or unreadable.
export function parseWeek(
  params: URLSearchParams,
  zone: string,
  now: DateTime = DateTime.now(),
): DateTime {
  const parsed = DateTime.fromISO(params.get(WEEK_PARAM) ?? '', { zone });
  return weekStart(parsed.isValid ? parsed : now.setZone(zone));
}

export function applyWeek(params: URLSearchParams, day: DateTime): URLSearchParams {
  const next = new URLSearchParams(params);
  next.set(WEEK_PARAM, dayKey(weekStart(day)));
  return next;
}

/// The seven Sunday-first days of the week that contains `day`.
export function weekDays(day: DateTime): DateTime[] {
  const start = weekStart(day);
  return Array.from({ length: 7 }, (_, offset) => start.plus({ days: offset }));
}

/// A span label for a week, folding away the month and year the two ends share.
export function weekLabel(week: readonly DateTime[]): string {
  if (week.length === 0) {
    return '';
  }

  const first = week[0];
  const last = week[week.length - 1];
  if (!first.hasSame(last, 'year')) {
    return `${first.toFormat('LLLL d, yyyy')} – ${last.toFormat('LLLL d, yyyy')}`;
  }
  if (!first.hasSame(last, 'month')) {
    return `${first.toFormat('LLLL d')} – ${last.toFormat('LLLL d, yyyy')}`;
  }
  return `${first.toFormat('LLLL d')} – ${last.toFormat('d, yyyy')}`;
}

export function dayKey(day: DateTime): string {
  return day.toISODate() ?? '';
}

/// The first and last calendar day an event covers, both inclusive. The stored
/// end is exclusive, so a date interval loses a day and a timed one loses any
/// part after its final midnight.
export function eventSpan(event: Event, zone: string): { first: DateTime; last: DateTime } {
  const first = instantOf(event.interval.start, zone).setZone(zone).startOf('day');

  if (isAllDay(event.interval)) {
    const end =
      event.interval.end.kind === 'date'
        ? DateTime.fromISO(event.interval.end.date, { zone }).startOf('day').minus({ days: 1 })
        : first;
    return { first, last: end < first ? first : end };
  }

  const end = instantOf(event.interval.end, zone)
    .setZone(zone)
    .minus({ milliseconds: 1 })
    .startOf('day');
  return { first, last: end < first ? first : end };
}

/// Lays the events that touch a week out as continuous bars, clipped to the
/// week and stacked into lanes so overlapping bars never share a row. One bar
/// per event per week is what lets a multi-day event span columns.
export function placeWeek(
  events: readonly CalendarEvent[],
  days: readonly DateTime[],
  zone: string,
): WeekLane[] {
  const first = days[0];
  const last = days[days.length - 1];

  const placements: EventPlacement[] = [];
  for (const event of events) {
    const span = eventSpan(event, zone);
    if (span.last < first || span.first > last) {
      continue;
    }

    const continuesBefore = span.first < first;
    const continuesAfter = span.last > last;
    const start = continuesBefore ? first : span.first;
    const end = continuesAfter ? last : span.last;

    placements.push({
      event,
      startIndex: daysBetween(first, start),
      endIndex: daysBetween(first, end),
      continuesBefore,
      continuesAfter,
      startTime: continuesBefore ? null : displayedClock(event.interval.start, zone),
      endTime: continuesAfter ? null : displayedClock(event.interval.end, zone),
    });
  }

  placements.sort(comparePlacements);
  return assignLanes(placements);
}

function comparePlacements(left: EventPlacement, right: EventPlacement): number {
  // all-day bars sit above timed ones, matching the server's sort order
  if (left.startTime === null || right.startTime === null) {
    if (left.startTime !== right.startTime) {
      return left.startTime === null ? -1 : 1;
    }
  } else if (left.startTime.toMillis() !== right.startTime.toMillis()) {
    return left.startTime.toMillis() - right.startTime.toMillis();
  }

  const leftSpan = left.endIndex - left.startIndex;
  const rightSpan = right.endIndex - right.startIndex;
  if (leftSpan !== rightSpan) {
    return rightSpan - leftSpan;
  }
  return left.event.title.localeCompare(right.event.title);
}

function assignLanes(placements: readonly EventPlacement[]): WeekLane[] {
  const lanes: WeekLane[] = [];
  for (const placement of placements) {
    const lane = lanes.find((candidate) =>
      candidate.every(
        (other) => other.endIndex < placement.startIndex || other.startIndex > placement.endIndex,
      ),
    );

    if (lane === undefined) {
      lanes.push([placement]);
    } else {
      lane.push(placement);
    }
  }
  return lanes;
}

function daysBetween(from: DateTime, to: DateTime): number {
  // a DST change makes the day difference 23 or 25 hours, so round to whole days
  return Math.round(to.diff(from, 'days').days);
}

/// The events the week view draws in its top band: every all-day event, plus
/// the timed ones whose span reaches into a second day.
export function spanningEvents(events: readonly CalendarEvent[], zone: string): CalendarEvent[] {
  return events.filter((event) => {
    if (isAllDay(event.interval)) {
      return true;
    }
    const span = eventSpan(event, zone);
    return !span.first.hasSame(span.last, 'day');
  });
}

export type TimedPlacement = {
  event: CalendarEvent;
  /// Minutes from local midnight, where 1440 is the midnight that ends the day.
  startMinute: number;
  endMinute: number;
  /// Zero-based column within the day.
  column: number;
  /// How many columns the overlapping cluster needs.
  columns: number;
};

/// The timed events that cover each day, laid out in side-by-side columns.
export function placeTimedWeek(
  events: readonly CalendarEvent[],
  days: readonly DateTime[],
  zone: string,
): TimedPlacement[][] {
  return days.map((day) => placeTimedDay(events, day, zone));
}

function placeTimedDay(
  events: readonly CalendarEvent[],
  day: DateTime,
  zone: string,
): TimedPlacement[] {
  const dayStart = day.startOf('day');
  const dayEnd = dayStart.plus({ days: 1 });

  const segments: TimedPlacement[] = [];
  for (const event of events) {
    if (isAllDay(event.interval)) {
      continue;
    }

    // a multi-day event is drawn in the top band instead
    const span = eventSpan(event, zone);
    if (!span.first.hasSame(span.last, 'day')) {
      continue;
    }

    const start = instantOf(event.interval.start, zone).setZone(zone);
    const end = instantOf(event.interval.end, zone).setZone(zone);
    if (!(start < dayEnd && end > dayStart)) {
      continue;
    }

    // wall-clock parts, so the segment still lines up with the hour labels on a
    // day the clocks shift
    const startMinute = start <= dayStart ? 0 : start.hour * 60 + start.minute;
    const endMinute = end >= dayEnd ? 1440 : end.hour * 60 + end.minute;
    if (endMinute <= startMinute) {
      continue;
    }

    segments.push({ event, startMinute, endMinute, column: 0, columns: 1 });
  }

  segments.sort(compareSegments);
  assignTimedColumns(segments);
  assignClusterWidths(segments);
  return segments;
}

function compareSegments(left: TimedPlacement, right: TimedPlacement): number {
  if (left.startMinute !== right.startMinute) {
    return left.startMinute - right.startMinute;
  }
  if (left.endMinute !== right.endMinute) {
    return right.endMinute - left.endMinute;
  }
  return left.event.title.localeCompare(right.event.title);
}

/// Gives each segment the first free column, opening a new one when they all
/// still hold an earlier segment.
function assignTimedColumns(segments: TimedPlacement[]): void {
  const columnEnds: number[] = [];
  for (const segment of segments) {
    const column = columnEnds.findIndex((end) => end <= segment.startMinute);
    if (column === -1) {
      segment.column = columnEnds.length;
      columnEnds.push(segment.endMinute);
    } else {
      segment.column = column;
      columnEnds[column] = segment.endMinute;
    }
  }
}

/// A cluster is a run of segments that overlap transitively; every segment in
/// it shares the cluster's column count so widths stay proportional.
function assignClusterWidths(segments: readonly TimedPlacement[]): void {
  let start = 0;
  let maxEnd = -1;
  for (let index = 0; index < segments.length; index += 1) {
    const segment = segments[index];
    if (segment.startMinute >= maxEnd) {
      if (index > start) {
        applyClusterWidth(segments, start, index);
        start = index;
      }
      maxEnd = segment.endMinute;
      continue;
    }
    maxEnd = Math.max(maxEnd, segment.endMinute);
  }
  if (start < segments.length) {
    applyClusterWidth(segments, start, segments.length);
  }
}

function applyClusterWidth(segments: readonly TimedPlacement[], from: number, to: number): void {
  let columns = 1;
  for (let index = from; index < to; index += 1) {
    columns = Math.max(columns, segments[index].column + 1);
  }
  for (let index = from; index < to; index += 1) {
    segments[index].columns = columns;
  }
}
