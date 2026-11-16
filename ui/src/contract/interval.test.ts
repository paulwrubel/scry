import { DateTime } from 'luxon';
import { describe, expect, it } from 'vitest';
import type { Interval } from '@/generated/Interval';
import { DEVICE_ZONE, dueToInputs } from './due';
import {
  defaultEndInputs,
  formatInterval,
  shiftEndInputs,
  intervalFromInputs,
  intervalToInputs,
  isAllDay,
  type IntervalInputs,
} from './interval';

const NOW = DateTime.fromISO('2026-11-06T15:00:00Z', { setZone: true });
const CHICAGO = 'America/Chicago';
const NEW_YORK = 'America/New_York';

function inputs(overrides: Partial<IntervalInputs> = {}): IntervalInputs {
  return { startDate: '2026-11-06', startTime: '', endDate: '', endTime: '', ...overrides };
}

function built(overrides: Partial<IntervalInputs> = {}): Interval {
  const result = intervalFromInputs(inputs(overrides));
  if (result.interval === null) {
    throw new Error(result.error);
  }
  return result.interval;
}

function dateInterval(start: string, end: string): Interval {
  return { start: { kind: 'date', date: start }, end: { kind: 'date', date: end } };
}

function zonedInterval(start: string, end: string, zone: string): Interval {
  return {
    start: { kind: 'zoned', datetime: start, zone },
    end: { kind: 'zoned', datetime: end, zone },
  };
}

describe('formatInterval', () => {
  it('labels a single all-day event by its day', () => {
    expect(formatInterval(dateInterval('2026-11-06', '2026-11-07'), NOW)).toBe('today');
  });

  it('shows the inclusive last day of a multi-day all-day event', () => {
    expect(formatInterval(dateInterval('2026-11-06', '2026-11-09'), NOW)).toBe('today – Nov 8');
  });

  it('labels a same-day timed event with both clock times', () => {
    const interval = zonedInterval('2026-11-06T10:00:00', '2026-11-06T11:00:00', CHICAGO);

    expect(formatInterval(interval, NOW, CHICAGO)).toBe('today 10:00 AM – 11:00 AM');
  });

  it('names the day of a timed event that ends on another day', () => {
    const interval = zonedInterval('2026-11-06T22:00:00', '2026-11-07T09:00:00', CHICAGO);

    expect(formatInterval(interval, NOW, CHICAGO)).toBe('today 10:00 PM – tomorrow 9:00 AM');
  });

  it('tags a zone the viewer is not in, keeping the civil time', () => {
    const interval = zonedInterval('2026-11-06T10:00:00', '2026-11-06T11:00:00', CHICAGO);

    expect(formatInterval(interval, NOW, NEW_YORK)).toBe(
      'today 10:00 AM – 11:00 AM (America/Chicago)',
    );
  });

  it('tags a floating event', () => {
    const interval: Interval = {
      start: { kind: 'floating', datetime: '2026-11-06T10:00:00' },
      end: { kind: 'floating', datetime: '2026-11-06T11:00:00' },
    };

    expect(formatInterval(interval, NOW, CHICAGO)).toBe('today 10:00 AM – 11:00 AM (floating)');
  });
});

describe('isAllDay', () => {
  it('is true only for date intervals', () => {
    expect(isAllDay(dateInterval('2026-11-06', '2026-11-07'))).toBe(true);
    expect(isAllDay(zonedInterval('2026-11-06T10:00:00', '2026-11-06T11:00:00', CHICAGO))).toBe(
      false,
    );
  });
});

describe('intervalFromInputs', () => {
  it('makes a date start a whole day', () => {
    expect(built()).toEqual(dateInterval('2026-11-06', '2026-11-07'));
  });

  it('spans an explicit inclusive end date', () => {
    expect(built({ endDate: '2026-11-08' })).toEqual(dateInterval('2026-11-06', '2026-11-09'));
  });

  it('defaults a timed start to one hour', () => {
    expect(intervalToInputs(built({ startTime: '09:00' }))).toEqual({
      startDate: '2026-11-06',
      startTime: '09:00',
      endDate: '2026-11-06',
      endTime: '10:00',
    });
  });

  it('treats an end time on its own as the same day', () => {
    expect(intervalToInputs(built({ startTime: '09:00', endTime: '17:30' }))).toEqual({
      startDate: '2026-11-06',
      startTime: '09:00',
      endDate: '2026-11-06',
      endTime: '17:30',
    });
  });

  it('rejects an interval that ends before it starts', () => {
    const result = intervalFromInputs(inputs({ startDate: '2026-11-08', endDate: '2026-11-07' }));

    expect(result.interval).toBeNull();
    expect(result.error).toBe('The end must be after the start.');
  });

  it('rejects a timed start without an end time', () => {
    const result = intervalFromInputs(inputs({ startTime: '09:00', endDate: '2026-11-07' }));

    expect(result.interval).toBeNull();
    expect(result.error).toContain('Choose an end time');
  });

  it('requires a start date', () => {
    const result = intervalFromInputs(inputs({ startDate: '' }));

    expect(result.interval).toBeNull();
    expect(result.error).toBe('Choose a start date.');
  });
});

describe('intervalToInputs', () => {
  it('turns a date end back into the inclusive last day', () => {
    expect(intervalToInputs(dateInterval('2026-11-06', '2026-11-09'))).toEqual({
      startDate: '2026-11-06',
      startTime: '',
      endDate: '2026-11-08',
      endTime: '',
    });
  });

  it('keeps the civil time of a zoned interval', () => {
    const interval = zonedInterval('2026-11-06T09:00:00', '2026-11-06T10:00:00', DEVICE_ZONE);

    expect(intervalToInputs(interval)).toEqual({
      startDate: '2026-11-06',
      startTime: '09:00',
      endDate: '2026-11-06',
      endTime: '10:00',
    });
  });

  it('reads a floating interval in the device zone', () => {
    const interval: Interval = {
      start: { kind: 'floating', datetime: '2026-11-06T09:00:00' },
      end: { kind: 'floating', datetime: '2026-11-06T10:00:00' },
    };

    expect(intervalToInputs(interval)).toEqual({
      startDate: '2026-11-06',
      startTime: '09:00',
      endDate: '2026-11-06',
      endTime: '10:00',
    });
  });
});

describe('round trip', () => {
  it('preserves a multi-day timed interval', () => {
    expect(
      intervalToInputs(built({ startTime: '22:00', endDate: '2026-11-07', endTime: '09:00' })),
    ).toEqual({
      startDate: '2026-11-06',
      startTime: '22:00',
      endDate: '2026-11-07',
      endTime: '09:00',
    });
  });

  it('agrees with dueToInputs on each end', () => {
    const value = built({ startTime: '09:00', endDate: '2026-11-07', endTime: '10:00' });

    expect(dueToInputs(value.start)).toEqual({ date: '2026-11-06', time: '09:00' });
    expect(dueToInputs(value.end)).toEqual({ date: '2026-11-07', time: '10:00' });
  });
});

describe('defaultEndInputs', () => {
  it('keeps an all-day event on the day it starts', () => {
    expect(defaultEndInputs('2026-11-06', '')).toEqual({
      endDate: '2026-11-06',
      endTime: '',
    });
  });

  it('runs one hour past a timed start', () => {
    expect(defaultEndInputs('2026-11-06', '09:00')).toEqual({
      endDate: '2026-11-06',
      endTime: '10:00',
    });
  });

  it('has nothing to show without a start date', () => {
    expect(defaultEndInputs('', '')).toEqual({ endDate: '', endTime: '' });
  });
});

describe('shiftEndInputs', () => {
  it('moves a timed end by the same minutes as the start', () => {
    expect(
      shiftEndInputs(
        { endDate: '2026-11-06', endTime: '10:00' },
        { startDate: '2026-11-06', startTime: '09:00' },
        { startDate: '2026-11-06', startTime: '09:30' },
      ),
    ).toEqual({ endDate: '2026-11-06', endTime: '10:30' });
  });

  it('moves a timed end to the next day when the start does', () => {
    expect(
      shiftEndInputs(
        { endDate: '2026-11-06', endTime: '10:00' },
        { startDate: '2026-11-06', startTime: '09:00' },
        { startDate: '2026-11-07', startTime: '09:00' },
      ),
    ).toEqual({ endDate: '2026-11-07', endTime: '10:00' });
  });

  it('moves an all-day end by the same days as the start', () => {
    expect(
      shiftEndInputs(
        { endDate: '2026-11-06', endTime: '' },
        { startDate: '2026-11-06', startTime: '' },
        { startDate: '2026-11-08', startTime: '' },
      ),
    ).toEqual({ endDate: '2026-11-08', endTime: '' });
  });

  it('has no answer when the start changes between all-day and timed', () => {
    expect(
      shiftEndInputs(
        { endDate: '2026-11-06', endTime: '' },
        { startDate: '2026-11-06', startTime: '' },
        { startDate: '2026-11-06', startTime: '09:00' },
      ),
    ).toBeNull();
  });

  it('has no answer when a timed end has no time to move', () => {
    expect(
      shiftEndInputs(
        { endDate: '2026-11-06', endTime: '' },
        { startDate: '2026-11-06', startTime: '09:00' },
        { startDate: '2026-11-06', startTime: '09:30' },
      ),
    ).toBeNull();
  });
});
