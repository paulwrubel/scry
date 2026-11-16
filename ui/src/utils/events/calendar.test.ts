import { DateTime } from 'luxon';
import { describe, expect, it } from 'vitest';
import type { Event } from '@/generated/Event';
import type { Interval } from '@/generated/Interval';
import type { EventPlacement, WeekLane } from './calendar';
import {
  MONTH_PARAM,
  applyMonth,
  dayKey,
  eventSpan,
  eventsOnDay,
  fitLanes,
  monthGrid,
  monthKey,
  monthLabel,
  parseMonth,
  placeWeek,
  shiftMonth,
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
