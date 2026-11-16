import { DateTime } from 'luxon';
import { instantOf, isAllDay, displayedClock } from '@/contract/interval';
import type { Event } from '@/generated/Event';

export const MONTH_PARAM = 'month';

export type Month = { year: number; month: number };

export type EventPlacement = {
  event: Event;
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
  events: readonly Event[],
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
