import { DateTime } from 'luxon';
import { describe, expect, it } from 'vitest';
import type { Interval } from '@/generated/Interval';
import type { RRule } from '@/generated/RRule';
import type { When } from '@/generated/When';
import { instantOf } from './interval';
import { expandRecurrence, formatRrule, parseRrule, type RecurrencePattern } from './recur';

const CHICAGO = 'America/Chicago';

function rule(text: string, exdates: When[] = []): RRule {
  return exdates.length > 0 ? { text, exdates } : { text };
}

function pattern(text: string): RecurrencePattern {
  const result = parseRrule(text);
  if (result.pattern === null) {
    throw new Error(result.error);
  }
  return result.pattern;
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

function utc(value: string): DateTime {
  return DateTime.fromISO(value, { setZone: true });
}

/// The resolved instant of a value as canonical UTC text, for compact asserts.
function instantIso(when: When, zone: string): string {
  return instantOf(when, zone).toUTC().toISO({ suppressMilliseconds: true }) ?? '';
}

/// The resolved instant of a value as UTC milliseconds, for compact asserts.
function instantMillis(when: When, zone: string): number {
  return instantOf(when, zone).toMillis();
}

function utcMillis(value: string): number {
  return utc(value).toMillis();
}

function startDates(intervals: Interval[]): string[] {
  return intervals.map((interval) =>
    interval.start.kind === 'date' ? interval.start.date : instantIso(interval.start, 'UTC'),
  );
}

describe('expandRecurrence zoned', () => {
  it('keeps a 09:00 wall clock across spring-forward while the instant shifts', () => {
    const series = zonedInterval('2026-03-01T09:00:00', '2026-03-01T10:00:00', CHICAGO);
    const got = expandRecurrence(
      series,
      rule('FREQ=WEEKLY'),
      utc('2026-03-01T00:00:00Z'),
      utc('2026-03-10T00:00:00Z'),
      CHICAGO,
    );

    expect(got).toHaveLength(2);
    expect(got[0].start).toEqual({
      kind: 'zoned',
      datetime: '2026-03-01T09:00:00',
      zone: CHICAGO,
    });
    expect(got[1].start).toEqual({
      kind: 'zoned',
      datetime: '2026-03-08T09:00:00',
      zone: CHICAGO,
    });
    expect(instantMillis(got[0].start, CHICAGO)).toBe(utcMillis('2026-03-01T15:00:00Z'));
    expect(instantMillis(got[1].start, CHICAGO)).toBe(utcMillis('2026-03-08T14:00:00Z'));
    expect(got[1].end).toEqual({
      kind: 'zoned',
      datetime: '2026-03-08T10:00:00',
      zone: CHICAGO,
    });
  });
});

describe('expandRecurrence date', () => {
  it('keeps local midnight, which moves in UTC across DST', () => {
    const got = expandRecurrence(
      dateInterval('2026-03-01', '2026-03-02'),
      rule('FREQ=WEEKLY'),
      utc('2026-02-01T00:00:00Z'),
      utc('2026-03-20T00:00:00Z'),
      CHICAGO,
    );

    expect(startDates(got)).toEqual(['2026-03-01', '2026-03-08', '2026-03-15']);
    expect(instantMillis(got[0].start, CHICAGO)).toBe(utcMillis('2026-03-01T06:00:00Z'));
    expect(instantMillis(got[2].start, CHICAGO)).toBe(utcMillis('2026-03-15T05:00:00Z'));
  });
});

describe('expandRecurrence floating', () => {
  it('keeps the wall clock while the resolved instant shifts', () => {
    const series: Interval = {
      start: { kind: 'floating', datetime: '2026-03-01T09:00:00' },
      end: { kind: 'floating', datetime: '2026-03-01T10:00:00' },
    };
    const got = expandRecurrence(
      series,
      rule('FREQ=WEEKLY'),
      utc('2026-03-01T00:00:00Z'),
      utc('2026-03-10T00:00:00Z'),
      CHICAGO,
    );

    expect(got).toHaveLength(2);
    expect(
      got.map((occurrence) =>
        occurrence.start.kind === 'floating' ? occurrence.start.datetime : '',
      ),
    ).toEqual(['2026-03-01T09:00:00', '2026-03-08T09:00:00']);
    expect(instantMillis(got[0].start, CHICAGO)).toBe(utcMillis('2026-03-01T15:00:00Z'));
    expect(instantMillis(got[1].start, CHICAGO)).toBe(utcMillis('2026-03-08T14:00:00Z'));
  });
});

describe('expandRecurrence instant', () => {
  it('repeats at the same absolute time every week', () => {
    const series: Interval = {
      start: { kind: 'instant', timestamp: '2026-03-01T15:00:00Z' },
      end: { kind: 'instant', timestamp: '2026-03-01T16:00:00Z' },
    };
    const got = expandRecurrence(
      series,
      rule('FREQ=WEEKLY'),
      utc('2026-03-01T00:00:00Z'),
      utc('2026-03-10T00:00:00Z'),
      CHICAGO,
    );

    expect(got.map((interval) => instantMillis(interval.start, CHICAGO))).toEqual([
      utcMillis('2026-03-01T15:00:00Z'),
      utcMillis('2026-03-08T15:00:00Z'),
    ]);
  });
});

describe('expandRecurrence bounds', () => {
  it('removes exactly the EXDATE occurrence', () => {
    const got = expandRecurrence(
      dateInterval('2026-11-01', '2026-11-02'),
      rule('FREQ=DAILY;COUNT=3', [{ kind: 'date', date: '2026-11-02' }]),
      utc('2026-11-01T00:00:00Z'),
      utc('2026-11-10T00:00:00Z'),
      'UTC',
    );

    expect(startDates(got)).toEqual(['2026-11-01', '2026-11-03']);
  });

  it('stops after COUNT occurrences including the base', () => {
    const got = expandRecurrence(
      dateInterval('2026-11-01', '2026-11-02'),
      rule('FREQ=DAILY;COUNT=3'),
      utc('2026-11-01T00:00:00Z'),
      utc('2026-12-01T00:00:00Z'),
      'UTC',
    );

    expect(startDates(got)).toEqual(['2026-11-01', '2026-11-02', '2026-11-03']);
  });

  it('includes the occurrence that falls on UNTIL', () => {
    const got = expandRecurrence(
      dateInterval('2026-11-01', '2026-11-02'),
      rule('FREQ=DAILY;UNTIL=20261103'),
      utc('2026-11-01T00:00:00Z'),
      utc('2026-12-01T00:00:00Z'),
      'UTC',
    );

    expect(startDates(got)).toEqual(['2026-11-01', '2026-11-02', '2026-11-03']);
  });

  it('still honours COUNT when the window starts late', () => {
    const got = expandRecurrence(
      dateInterval('2026-01-01', '2026-01-02'),
      rule('FREQ=DAILY;COUNT=10'),
      utc('2026-01-08T00:00:00Z'),
      utc('2026-01-15T00:00:00Z'),
      'UTC',
    );

    expect(startDates(got)).toEqual(['2026-01-08', '2026-01-09', '2026-01-10']);
  });
});

describe('expandRecurrence BY rules', () => {
  it('combines INTERVAL, BYDAY, and WKST for weekly', () => {
    const got = expandRecurrence(
      dateInterval('2026-03-04', '2026-03-05'),
      rule('FREQ=WEEKLY;INTERVAL=2;BYDAY=SU,WE;WKST=SU'),
      utc('2026-03-01T00:00:00Z'),
      utc('2026-04-01T00:00:00Z'),
      'UTC',
    );

    expect(startDates(got)).toEqual(['2026-03-04', '2026-03-15', '2026-03-18', '2026-03-29']);
  });

  it('counts a negative BYMONTHDAY back from the month end', () => {
    const got = expandRecurrence(
      dateInterval('2026-01-31', '2026-02-01'),
      rule('FREQ=MONTHLY;BYMONTHDAY=-1'),
      utc('2026-01-01T00:00:00Z'),
      utc('2026-04-01T00:00:00Z'),
      'UTC',
    );

    expect(startDates(got)).toEqual(['2026-01-31', '2026-02-28', '2026-03-31']);
  });

  it('selects the last Friday of each month', () => {
    const got = expandRecurrence(
      dateInterval('2026-01-30', '2026-01-31'),
      rule('FREQ=MONTHLY;BYDAY=-1FR'),
      utc('2026-01-01T00:00:00Z'),
      utc('2026-04-01T00:00:00Z'),
      'UTC',
    );

    expect(startDates(got)).toEqual(['2026-01-30', '2026-02-27', '2026-03-27']);
  });

  it('picks one date per year from BYMONTH and BYMONTHDAY', () => {
    const got = expandRecurrence(
      dateInterval('2026-03-15', '2026-03-16'),
      rule('FREQ=YEARLY;BYMONTH=3;BYMONTHDAY=15'),
      utc('2026-01-01T00:00:00Z'),
      utc('2029-01-01T00:00:00Z'),
      'UTC',
    );

    expect(startDates(got)).toEqual(['2026-03-15', '2027-03-15', '2028-03-15']);
  });

  it('reads an ordinal YEARLY BYDAY against the whole year', () => {
    const got = expandRecurrence(
      dateInterval('2026-01-01', '2026-01-02'),
      rule('FREQ=YEARLY;BYDAY=-1SU'),
      utc('2026-01-01T00:00:00Z'),
      utc('2028-01-01T00:00:00Z'),
      'UTC',
    );

    expect(startDates(got)).toEqual(['2026-01-01', '2026-12-27', '2027-12-26']);
  });
});

describe('parseRrule', () => {
  it('rejects rules outside the supported subset', () => {
    const cases: Array<[string, string]> = [
      ['FREQ=DAILY;FOO=1', 'FOO'],
      ['COUNT=2', 'FREQ'],
      ['FREQ=DAILY;COUNT=2;UNTIL=20260101', 'COUNT and UNTIL'],
      ['FREQ=DAILY;INTERVAL=0', 'INTERVAL'],
      ['FREQ=WEEKLY;BYDAY=1MO', 'ordinal'],
      ['FREQ=DAILY;UNTIL=2026-01-01', 'YYYYMMDD'],
    ];

    for (const [text, expected] of cases) {
      const result = parseRrule(text);
      expect(result.pattern).toBeNull();
      expect(result.error).toContain(expected);
    }
  });

  it('accepts the RRULE prefix in either case', () => {
    expect(parseRrule('RRULE:FREQ=DAILY').error).toBeNull();
    expect(parseRrule('rrule:freq=daily').error).toBeNull();
  });
});

describe('formatRrule', () => {
  it('emits canonical order and keeps ordinal BYDAY tokens', () => {
    const formatted = formatRrule(
      pattern(
        'RRULE:FREQ=MONTHLY;INTERVAL=2;BYMONTH=3,11;BYMONTHDAY=-1;BYDAY=1MO,-1FR;WKST=SU;COUNT=5',
      ),
    );

    expect(formatted).toBe(
      'FREQ=MONTHLY;INTERVAL=2;BYMONTH=3,11;BYMONTHDAY=-1;BYDAY=1MO,-1FR;WKST=SU;COUNT=5',
    );
  });

  it('drops defaults and normalizes input casing', () => {
    expect(formatRrule(pattern('rrule:freq=weekly;wkst=mo'))).toBe('FREQ=WEEKLY');
  });

  it('round-trips both UNTIL forms', () => {
    for (const text of ['FREQ=DAILY;UNTIL=20261231', 'FREQ=DAILY;UNTIL=20301231T235959Z']) {
      const first = pattern(text);
      expect(pattern(formatRrule(first))).toEqual(first);
    }
  });

  it('round-trips a rule with every field', () => {
    const first = pattern(
      'FREQ=YEARLY;INTERVAL=2;BYMONTH=1,7;BYMONTHDAY=1,-1;BYDAY=MO,-1FR;WKST=SU;UNTIL=20301231T235959Z',
    );

    expect(pattern(formatRrule(first))).toEqual(first);
  });
});
