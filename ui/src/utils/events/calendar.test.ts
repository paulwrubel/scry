import { DateTime } from 'luxon';
import { describe, expect, it } from 'vitest';
import type { Event } from '@/generated/Event';
import type { Interval } from '@/generated/Interval';
import type { When } from '@/generated/When';
import type { EventPlacement, WeekLane } from './calendar';
import {
  MONTH_PARAM,
  VIEW_PARAM,
  WEEK_PARAM,
  applyCalendarView,
  applyMonth,
  applyWeek,
  dayKey,
  eventKey,
  eventPath,
  eventSpan,
  eventsOnDay,
  expandEvents,
  fitLanes,
  monthGrid,
  monthKey,
  monthLabel,
  parseCalendarView,
  parseMonth,
  parseWeek,
  placeTimedWeek,
  placeWeek,
  shiftMonth,
  spanningEvents,
  weekDays,
  weekLabel,
  weekStart,
} from './calendar';

const ZONE = 'UTC';
const NOW = DateTime.fromISO('2026-11-06T15:00:00Z', { setZone: true });
const CREATED_AT = '2026-01-01T00:00:00Z';

function params(value?: string): URLSearchParams {
  return new URLSearchParams(value === undefined ? '' : `${MONTH_PARAM}=${value}`);
}

function dateInterval(start: string, end: string): Interval {
  return { start: { kind: 'date', date: start }, end: { kind: 'date', date: end } };
}

function timedInterval(start: string, end: string): Interval {
  return {
    start: { kind: 'zoned', datetime: start, zone: ZONE },
    end: { kind: 'zoned', datetime: end, zone: ZONE },
  };
}

function event(id: string, interval: Interval): Event {
  return {
    id,
    project_id: 'project-1',
    title: id,
    description: null,
    interval,
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
  };
}

function recurringEvent(id: string, interval: Interval, text: string, exdates: When[] = []): Event {
  return { ...event(id, interval), recurrence: exdates.length > 0 ? { text, exdates } : { text } };
}

function day(value: string): DateTime {
  return DateTime.fromISO(value, { zone: ZONE });
}

describe('parseMonth', () => {
  it('reads an explicit month', () => {
    expect(parseMonth(params('2026-02'), NOW)).toEqual({ year: 2026, month: 2 });
  });

  it('falls back to the current month when the parameter is missing or invalid', () => {
    expect(parseMonth(params(), NOW)).toEqual({ year: 2026, month: 11 });
    expect(parseMonth(params('nonsense'), NOW)).toEqual({ year: 2026, month: 11 });
  });
});

describe('applyMonth', () => {
  it('keeps the other parameters and sets the month', () => {
    const next = applyMonth(new URLSearchParams('date=2026-11-06'), { year: 2027, month: 1 });

    expect(next.toString()).toBe('date=2026-11-06&month=2027-01');
  });
});

describe('monthKey', () => {
  it('pads a single-digit month', () => {
    expect(monthKey({ year: 2026, month: 1 })).toBe('2026-01');
    expect(monthKey({ year: 2026, month: 11 })).toBe('2026-11');
  });
});

describe('shiftMonth', () => {
  it('crosses year boundaries in both directions', () => {
    expect(shiftMonth({ year: 2026, month: 12 }, 1)).toEqual({ year: 2027, month: 1 });
    expect(shiftMonth({ year: 2026, month: 1 }, -1)).toEqual({ year: 2025, month: 12 });
  });
});

describe('monthLabel', () => {
  it('names the month and year', () => {
    expect(monthLabel({ year: 2026, month: 11 })).toBe('November 2026');
  });
});

describe('monthGrid', () => {
  it('returns whole Sunday-first weeks that cover the month', () => {
    const weeks = monthGrid({ year: 2026, month: 11 }, ZONE);

    expect(weeks.every((week) => week.length === 7)).toBe(true);
    expect(weeks[0][0].toISODate()).toBe('2026-11-01');
    expect(weeks.at(-1)?.at(-1)?.toISODate()).toBe('2026-12-05');
    expect(weeks[0][0].weekday).toBe(7);
  });

  it('starts in the previous month when the first falls mid-week', () => {
    const weeks = monthGrid({ year: 2026, month: 4 }, ZONE);

    expect(weeks[0][0].toISODate()).toBe('2026-03-29');
    expect(weeks[0][0].weekday).toBe(7);
  });
});

describe('eventSpan', () => {
  it('covers one day for a single all-day event', () => {
    const span = eventSpan(event('a', dateInterval('2026-11-06', '2026-11-07')), ZONE);

    expect(dayKey(span.first)).toBe('2026-11-06');
    expect(dayKey(span.last)).toBe('2026-11-06');
  });

  it('covers the inclusive last day of a multi-day all-day event', () => {
    const span = eventSpan(event('a', dateInterval('2026-11-06', '2026-11-09')), ZONE);

    expect(dayKey(span.first)).toBe('2026-11-06');
    expect(dayKey(span.last)).toBe('2026-11-08');
  });

  it('covers one day for a same-day timed event', () => {
    const span = eventSpan(
      event('a', timedInterval('2026-11-06T09:00:00', '2026-11-06T10:00:00')),
      ZONE,
    );

    expect(dayKey(span.first)).toBe('2026-11-06');
    expect(dayKey(span.last)).toBe('2026-11-06');
  });

  it('covers both days for a timed event that crosses midnight', () => {
    const span = eventSpan(
      event('a', timedInterval('2026-11-06T23:00:00', '2026-11-07T01:00:00')),
      ZONE,
    );

    expect(dayKey(span.first)).toBe('2026-11-06');
    expect(dayKey(span.last)).toBe('2026-11-07');
  });

  it('does not spill into the day a timed event ends exactly at midnight', () => {
    const span = eventSpan(
      event('a', timedInterval('2026-11-06T22:00:00', '2026-11-07T00:00:00')),
      ZONE,
    );

    expect(dayKey(span.last)).toBe('2026-11-06');
  });
});

describe('placeWeek', () => {
  function week(start: string): DateTime[] {
    return Array.from({ length: 7 }, (_, index) => day(start).plus({ days: index }));
  }

  it('draws a multi-day event as one bar spanning its days', () => {
    const lanes = placeWeek(
      [event('offsite', dateInterval('2026-11-02', '2026-11-05'))],
      week('2026-11-01'),
      ZONE,
    );

    expect(lanes).toHaveLength(1);
    expect(lanes[0][0]).toMatchObject({
      startIndex: 1,
      endIndex: 3,
      continuesBefore: false,
      continuesAfter: false,
      startTime: null,
      endTime: null,
    });
  });

  it('clips a bar to the week and marks the continuation', () => {
    const lanes = placeWeek(
      [event('trip', dateInterval('2026-11-06', '2026-11-10'))],
      week('2026-11-01'),
      ZONE,
    );

    expect(lanes[0][0]).toMatchObject({
      startIndex: 5,
      endIndex: 6,
      continuesBefore: false,
      continuesAfter: true,
    });
  });

  it('marks a bar that began before the week', () => {
    const lanes = placeWeek(
      [event('trip', dateInterval('2026-10-30', '2026-11-03'))],
      week('2026-11-01'),
      ZONE,
    );

    expect(lanes[0][0]).toMatchObject({
      startIndex: 0,
      endIndex: 1,
      continuesBefore: true,
      continuesAfter: false,
    });
  });

  it('puts all-day bars above timed ones in separate lanes when they overlap', () => {
    const lanes = placeWeek(
      [
        event('standup', timedInterval('2026-11-02T09:00:00', '2026-11-02T10:00:00')),
        event('offsite', dateInterval('2026-11-02', '2026-11-04')),
      ],
      week('2026-11-01'),
      ZONE,
    );

    expect(lanes).toHaveLength(2);
    expect(lanes[0][0].event.id).toBe('offsite');
    expect(lanes[1][0].event.id).toBe('standup');
  });

  it('shares a lane between events that do not overlap', () => {
    const lanes = placeWeek(
      [
        event('first', dateInterval('2026-11-02', '2026-11-03')),
        event('second', dateInterval('2026-11-04', '2026-11-05')),
      ],
      week('2026-11-01'),
      ZONE,
    );

    expect(lanes).toHaveLength(1);
    expect(lanes[0].map((placement) => placement.event.id)).toEqual(['first', 'second']);
  });

  it('carries both clock times on the segment that starts and ends the event', () => {
    const lanes = placeWeek(
      [event('standup', timedInterval('2026-11-02T09:00:00', '2026-11-02T10:00:00'))],
      week('2026-11-01'),
      ZONE,
    );

    expect(lanes[0][0].startTime?.toFormat('HH:mm')).toBe('09:00');
    expect(lanes[0][0].endTime?.toFormat('HH:mm')).toBe('10:00');
  });

  it('omits the end time when the event continues into the next week', () => {
    const lanes = placeWeek(
      [event('night', timedInterval('2026-11-06T23:00:00', '2026-11-08T01:00:00'))],
      week('2026-11-01'),
      ZONE,
    );

    expect(lanes[0][0].startTime?.toFormat('HH:mm')).toBe('23:00');
    expect(lanes[0][0].continuesAfter).toBe(true);
    expect(lanes[0][0].endTime).toBeNull();
  });

  it('shows only the end time on the segment where the event finishes', () => {
    const lanes = placeWeek(
      [event('night', timedInterval('2026-11-06T23:00:00', '2026-11-08T01:00:00'))],
      week('2026-11-08'),
      ZONE,
    );

    expect(lanes[0][0].continuesBefore).toBe(true);
    expect(lanes[0][0].startTime).toBeNull();
    expect(lanes[0][0].endTime?.toFormat('HH:mm')).toBe('01:00');
  });

  it('drops events outside the week', () => {
    expect(
      placeWeek(
        [event('later', dateInterval('2026-11-10', '2026-11-11'))],
        week('2026-11-01'),
        ZONE,
      ),
    ).toEqual([]);
  });
});

describe('fitLanes', () => {
  function placement(startIndex: number, endIndex: number): EventPlacement {
    return {
      event: event(`event-${startIndex}-${endIndex}`, dateInterval('2026-11-01', '2026-11-02')),
      startIndex,
      endIndex,
      continuesBefore: false,
      continuesAfter: false,
      startTime: null,
      endTime: null,
    };
  }

  function lane(startIndex: number, endIndex = startIndex): WeekLane {
    return [placement(startIndex, endIndex)];
  }

  it('keeps every lane when the height is unknown', () => {
    const result = fitLanes([lane(0), lane(1), lane(2)], null);

    expect(result.visible).toHaveLength(3);
    expect(result.hiddenByDay).toEqual([0, 0, 0, 0, 0, 0, 0]);
  });

  it('keeps every lane when they fit', () => {
    const result = fitLanes([lane(0), lane(1)], 3);

    expect(result.visible).toHaveLength(2);
    expect(result.hiddenByDay).toEqual([0, 0, 0, 0, 0, 0, 0]);
  });

  it('hides the tail and counts it against each day it covers', () => {
    const result = fitLanes([lane(0), lane(2), lane(2, 4)], 2);

    expect(result.visible).toHaveLength(1);
    expect(result.hiddenByDay).toEqual([0, 0, 2, 1, 1, 0, 0]);
  });

  it('adds up the bars hidden on the same day', () => {
    const result = fitLanes([lane(0), lane(1), lane(2), lane(2)], 3);

    expect(result.hiddenByDay).toEqual([0, 0, 2, 0, 0, 0, 0]);
  });

  it('keeps a single slot for the count when only one row fits', () => {
    const result = fitLanes([lane(0), lane(2)], 1);

    expect(result.visible).toEqual([]);
    expect(result.hiddenByDay).toEqual([1, 0, 1, 0, 0, 0, 0]);
  });
});

describe('eventsOnDay', () => {
  it('keeps the events covering the day and puts all-day first', () => {
    const events = [
      event('standup', timedInterval('2026-11-06T09:00:00', '2026-11-06T10:00:00')),
      event('offsite', dateInterval('2026-11-05', '2026-11-08')),
      event('later', timedInterval('2026-11-06T14:00:00', '2026-11-06T15:00:00')),
    ];

    expect(eventsOnDay(events, day('2026-11-06'), ZONE).map((item) => item.id)).toEqual([
      'offsite',
      'standup',
      'later',
    ]);
  });

  it('covers a multi-day event through its inclusive last day', () => {
    const events = [event('offsite', dateInterval('2026-11-05', '2026-11-08'))];

    expect(eventsOnDay(events, day('2026-11-07'), ZONE).map((item) => item.id)).toEqual([
      'offsite',
    ]);
    expect(eventsOnDay(events, day('2026-11-08'), ZONE)).toEqual([]);
  });

  it('drops days no event covers', () => {
    const events = [event('standup', timedInterval('2026-11-06T09:00:00', '2026-11-06T10:00:00'))];

    expect(eventsOnDay(events, day('2026-11-05'), ZONE)).toEqual([]);
  });
});

describe('parseCalendarView', () => {
  it('defaults to the month view when the parameter is missing', () => {
    expect(parseCalendarView(new URLSearchParams())).toBe('month');
  });

  it('reads an explicit month view', () => {
    expect(parseCalendarView(new URLSearchParams(`${VIEW_PARAM}=month`))).toBe('month');
  });

  it('reads the week view', () => {
    expect(parseCalendarView(new URLSearchParams(`${VIEW_PARAM}=week`))).toBe('week');
  });

  it('falls back to the month view for an unrecognised value', () => {
    expect(parseCalendarView(new URLSearchParams(`${VIEW_PARAM}=nonsense`))).toBe('month');
  });
});

describe('applyCalendarView', () => {
  it('sets the month view alongside the other parameters', () => {
    const next = applyCalendarView(new URLSearchParams('month=2026-11'), 'month');

    expect(next.toString()).toBe('month=2026-11&view=month');
  });

  it('sets the week view alongside the other parameters', () => {
    const next = applyCalendarView(new URLSearchParams('month=2026-11'), 'week');

    expect(next.toString()).toBe('month=2026-11&view=week');
  });

  it('does not mutate the input parameters', () => {
    const input = new URLSearchParams('month=2026-11');

    applyCalendarView(input, 'week');

    expect(input.toString()).toBe('month=2026-11');
  });
});

describe('weekStart', () => {
  it('rewinds a mid-week day to the Sunday before it', () => {
    expect(dayKey(weekStart(day('2026-11-06')))).toBe('2026-11-01');
  });

  it('leaves a Sunday where it is', () => {
    expect(dayKey(weekStart(day('2026-11-01')))).toBe('2026-11-01');
  });
});

describe('parseWeek', () => {
  it('anchors an explicit date to the Sunday of its week', () => {
    expect(dayKey(parseWeek(new URLSearchParams(`${WEEK_PARAM}=2026-11-04`), ZONE, NOW))).toBe(
      '2026-11-01',
    );
  });

  it('anchors a missing week to the current week', () => {
    expect(dayKey(parseWeek(new URLSearchParams(), ZONE, NOW))).toBe('2026-11-01');
  });

  it('anchors an unreadable week to the current week', () => {
    expect(dayKey(parseWeek(new URLSearchParams(`${WEEK_PARAM}=nonsense`), ZONE, NOW))).toBe(
      '2026-11-01',
    );
  });
});

describe('applyWeek', () => {
  it('sets the Sunday that starts the given day and keeps the other parameters', () => {
    const next = applyWeek(new URLSearchParams('month=2026-11'), day('2026-11-04'));

    expect(next.get(WEEK_PARAM)).toBe('2026-11-01');
    expect(next.get(MONTH_PARAM)).toBe('2026-11');
  });
});

describe('weekLabel', () => {
  function weekOf(start: string): DateTime[] {
    return Array.from({ length: 7 }, (_, index) => day(start).plus({ days: index }));
  }

  it('folds a week that stays inside one month', () => {
    expect(weekLabel(weekOf('2026-11-01'))).toBe('November 1 – 7, 2026');
  });

  it('keeps both month names when the week crosses one', () => {
    expect(weekLabel(weekOf('2026-11-29'))).toBe('November 29 – December 5, 2026');
  });

  it('keeps both years when the week crosses one', () => {
    expect(weekLabel(weekOf('2025-12-28'))).toBe('December 28, 2025 – January 3, 2026');
  });
});

describe('spanningEvents', () => {
  it('keeps the all-day events and the timed one that crosses midnight', () => {
    const events = [
      event('holiday', dateInterval('2026-11-02', '2026-11-03')),
      event('offsite', dateInterval('2026-11-02', '2026-11-05')),
      event('standup', timedInterval('2026-11-02T09:00:00', '2026-11-02T10:00:00')),
      event('night', timedInterval('2026-11-06T23:00:00', '2026-11-07T01:00:00')),
    ];

    expect(spanningEvents(events, ZONE).map((item) => item.id)).toEqual([
      'holiday',
      'offsite',
      'night',
    ]);
  });
});

describe('placeTimedWeek', () => {
  const week = Array.from({ length: 7 }, (_, index) => day('2026-11-01').plus({ days: index }));

  function placementsByDay(events: Event[]): ReturnType<typeof placeTimedWeek> {
    return placeTimedWeek(events, week, ZONE);
  }

  it('returns one list per day', () => {
    expect(placementsByDay([])).toHaveLength(7);
  });

  it('places a timed event on its day and leaves the other days empty', () => {
    const placements = placementsByDay([
      event('standup', timedInterval('2026-11-02T09:00:00', '2026-11-02T10:00:00')),
    ]);

    expect(placements[1]).toMatchObject([
      { event: { id: 'standup' }, startMinute: 540, endMinute: 600, column: 0, columns: 1 },
    ]);
    expect(
      placements.filter((dayPlacements, index) => index !== 1 && dayPlacements.length > 0),
    ).toEqual([]);
  });

  it('gives overlapping events separate columns in a shared cluster', () => {
    const placements = placementsByDay([
      event('first', timedInterval('2026-11-02T09:00:00', '2026-11-02T10:00:00')),
      event('second', timedInterval('2026-11-02T09:30:00', '2026-11-02T10:30:00')),
    ]);

    expect(placements[1]).toMatchObject([
      { event: { id: 'first' }, column: 0, columns: 2 },
      { event: { id: 'second' }, column: 1, columns: 2 },
    ]);
  });

  it('shares a column between events that only touch', () => {
    const placements = placementsByDay([
      event('first', timedInterval('2026-11-02T09:00:00', '2026-11-02T10:00:00')),
      event('second', timedInterval('2026-11-02T10:00:00', '2026-11-02T11:00:00')),
    ]);

    expect(placements[1]).toMatchObject([
      { event: { id: 'first' }, column: 0, columns: 1 },
      { event: { id: 'second' }, column: 0, columns: 1 },
    ]);
  });

  it('reuses a freed column in the middle of an overlapping chain', () => {
    const placements = placementsByDay([
      event('first', timedInterval('2026-11-02T09:00:00', '2026-11-02T10:00:00')),
      event('second', timedInterval('2026-11-02T09:30:00', '2026-11-02T10:30:00')),
      event('third', timedInterval('2026-11-02T10:00:00', '2026-11-02T11:00:00')),
    ]);

    expect(placements[1]).toMatchObject([
      { event: { id: 'first' }, column: 0, columns: 2 },
      { event: { id: 'second' }, column: 1, columns: 2 },
      { event: { id: 'third' }, column: 0, columns: 2 },
    ]);
  });

  it('runs an event that ends at midnight to the bottom of the day', () => {
    const placements = placementsByDay([
      event('wind-down', timedInterval('2026-11-02T22:00:00', '2026-11-03T00:00:00')),
    ]);

    expect(placements[1]).toMatchObject([
      { event: { id: 'wind-down' }, startMinute: 1320, endMinute: 1440 },
    ]);
  });

  it('leaves all-day and overnight events out of the timed columns', () => {
    const placements = placementsByDay([
      event('offsite', dateInterval('2026-11-02', '2026-11-05')),
      event('night', timedInterval('2026-11-02T23:00:00', '2026-11-03T01:00:00')),
    ]);

    expect(placements.every((dayPlacements) => dayPlacements.length === 0)).toBe(true);
  });
});

describe('weekDays', () => {
  it('returns seven days starting on the Sunday of that week', () => {
    const days = weekDays(day('2026-11-04'));

    expect(days).toHaveLength(7);
    expect(dayKey(days[0])).toBe('2026-11-01');
    expect(dayKey(days[6])).toBe('2026-11-07');
    expect(days.every((item) => item.hour === 0)).toBe(true);
  });
});

describe('expandEvents', () => {
  const from = day('2026-11-01');
  const to = day('2026-11-08');

  it('shifts an occurrence of a series that began before the window', () => {
    const expanded = expandEvents(
      [recurringEvent('weekly', dateInterval('2026-10-05', '2026-10-06'), 'FREQ=WEEKLY')],
      from,
      to,
      ZONE,
    );

    expect(expanded).toHaveLength(1);
    expect(expanded[0].id).toBe('weekly');
    expect(expanded[0].occurrenceStart).toEqual({ kind: 'date', date: '2026-11-02' });
    expect(expanded[0].interval).toEqual(dateInterval('2026-11-02', '2026-11-03'));
  });

  it('keeps two occurrences of one series that fall inside the window', () => {
    const expanded = expandEvents(
      [
        recurringEvent(
          'twice',
          dateInterval('2026-10-05', '2026-10-06'),
          'FREQ=WEEKLY;BYDAY=MO,TH',
        ),
      ],
      from,
      to,
      ZONE,
    );

    expect(expanded.map((item) => item.occurrenceStart)).toEqual([
      { kind: 'date', date: '2026-11-02' },
      { kind: 'date', date: '2026-11-05' },
    ]);
  });

  it('removes exactly the occurrence named by an EXDATE', () => {
    const expanded = expandEvents(
      [
        recurringEvent('daily', dateInterval('2026-11-02', '2026-11-03'), 'FREQ=DAILY;COUNT=5', [
          { kind: 'date', date: '2026-11-03' },
        ]),
      ],
      from,
      to,
      ZONE,
    );

    expect(expanded.map((item) => item.occurrenceStart)).toEqual([
      { kind: 'date', date: '2026-11-02' },
      { kind: 'date', date: '2026-11-04' },
      { kind: 'date', date: '2026-11-05' },
      { kind: 'date', date: '2026-11-06' },
    ]);
  });

  it('returns a non-recurring event unchanged', () => {
    const plain = event('plain', dateInterval('2026-11-03', '2026-11-04'));

    expect(expandEvents([plain], from, to, ZONE)[0]).toBe(plain);
  });
});

describe('eventKey', () => {
  it('differs for two occurrences of one series', () => {
    const expanded = expandEvents(
      [
        recurringEvent(
          'twice',
          dateInterval('2026-10-05', '2026-10-06'),
          'FREQ=WEEKLY;BYDAY=MO,TH',
        ),
      ],
      day('2026-11-01'),
      day('2026-11-08'),
      ZONE,
    );

    expect(expanded).toHaveLength(2);
    expect(eventKey(expanded[0]).startsWith('twice:')).toBe(true);
    expect(eventKey(expanded[0])).not.toBe(eventKey(expanded[1]));
  });
});

describe('eventPath', () => {
  it('adds an encoded occurrence and keeps the existing parameters', () => {
    const [occurrence] = expandEvents(
      [recurringEvent('weekly', dateInterval('2026-10-05', '2026-10-06'), 'FREQ=WEEKLY')],
      day('2026-11-01'),
      day('2026-11-08'),
      ZONE,
    );

    const path = eventPath('project-1', occurrence, '?month=2026-11&view=week');
    const [base, query] = path.split('?');
    const queryParams = new URLSearchParams(query);

    expect(base).toBe('/projects/project-1/events/weekly');
    expect(queryParams.get('month')).toBe('2026-11');
    expect(queryParams.get('view')).toBe('week');
    expect(queryParams.get('occurrence')).toBe(JSON.stringify(occurrence.occurrenceStart));
    expect(path).toContain('%22kind%22');
  });
});
