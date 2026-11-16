import { DateTime } from 'luxon';
import { describe, expect, it } from 'vitest';
import type { Task } from '@/generated/Task';
import type { When } from '@/generated/When';
import {
  DEVICE_ZONE,
  deadline,
  dueFromInputs,
  dueToInputs,
  formatDue,
  isOverdue,
  sameDue,
} from './due';

const NOW = DateTime.fromISO('2026-11-06T15:00:00Z', { setZone: true });
const AFTER = DateTime.fromISO('2027-01-01T00:00:00Z', { setZone: true });
const CHICAGO = 'America/Chicago';
const NEW_YORK = 'America/New_York';

function dateWhen(date: string): When {
  return { kind: 'date', date };
}

function floatingWhen(datetime: string): When {
  return { kind: 'floating', datetime };
}

function zonedWhen(datetime: string, zone: string): When {
  return { kind: 'zoned', datetime, zone };
}

describe('deadline', () => {
  it('resolves a date to the next local midnight', () => {
    expect(deadline(dateWhen('2026-11-06'), 'UTC').toUTC().toISO()).toBe(
      '2026-11-07T00:00:00.000Z',
    );
  });

  it('resolves a floating time in the viewer zone', () => {
    expect(deadline(floatingWhen('2026-11-06T17:00:00'), 'UTC').toUTC().toISO()).toBe(
      '2026-11-06T17:00:00.000Z',
    );
  });

  it('resolves a zoned value in its own zone', () => {
    const when = zonedWhen('2026-11-06T17:00:00', CHICAGO);
    expect(deadline(when).toUTC().toISO()).toBe('2026-11-06T23:00:00.000Z');
  });

  it('uses the instant timestamp as-is', () => {
    const timestamp = '2026-11-06T23:00:00Z';
    expect(deadline({ kind: 'instant', timestamp }).toUTC().toISO()).toBe(
      '2026-11-06T23:00:00.000Z',
    );
  });
});

describe('isOverdue', () => {
  const task: Pick<Task, 'due' | 'state'> = {
    due: dateWhen('2026-11-06'),
    state: 'open',
  };

  it('is false without a due value', () => {
    expect(isOverdue({ due: null, state: 'open' }, AFTER, 'UTC')).toBe(false);
  });

  it('is never true for a closed task', () => {
    expect(isOverdue({ due: task.due, state: 'closed' }, AFTER, 'UTC')).toBe(false);
  });

  it('is false before the deadline and true at it', () => {
    expect(
      isOverdue(task, DateTime.fromISO('2026-11-06T23:59:59Z', { setZone: true }), 'UTC'),
    ).toBe(false);
    expect(
      isOverdue(task, DateTime.fromISO('2026-11-07T00:00:00Z', { setZone: true }), 'UTC'),
    ).toBe(true);
  });
});

describe('formatDue', () => {
  it('labels near days relatively', () => {
    expect(formatDue(dateWhen('2026-11-06'), NOW, CHICAGO)).toBe('today');
    expect(formatDue(dateWhen('2026-11-05'), NOW, CHICAGO)).toBe('yesterday');
    expect(formatDue(dateWhen('2026-11-07'), NOW, CHICAGO)).toBe('tomorrow');
    expect(formatDue(dateWhen('2026-11-09'), NOW, CHICAGO)).toBe('in 3d');
  });

  it('falls back to an absolute date outside the window', () => {
    expect(formatDue(dateWhen('2026-11-03'), NOW, CHICAGO)).toBe('Nov 3');
    expect(formatDue(dateWhen('2027-01-02'), NOW, CHICAGO)).toBe('Jan 2, 2027');
  });

  it('omits the zone label when a zoned value matches the viewer zone', () => {
    const when = zonedWhen('2026-11-06T17:00:00', CHICAGO);
    expect(formatDue(when, NOW, CHICAGO)).toBe('today 5:00 PM');
  });

  it('labels a zoned value in another zone', () => {
    const when = zonedWhen('2026-11-06T17:00:00', CHICAGO);
    expect(formatDue(when, NOW, NEW_YORK)).toBe('today 5:00 PM (America/Chicago)');
  });

  it('displays an instant in the viewer zone', () => {
    const when: When = { kind: 'instant', timestamp: '2026-11-06T23:00:00Z' };
    expect(formatDue(when, NOW, CHICAGO)).toBe('today 5:00 PM');
    expect(formatDue(when, NOW, NEW_YORK)).toBe('today 6:00 PM');
  });

  it('marks a floating value', () => {
    const when = floatingWhen('2026-11-06T17:00:00');
    expect(formatDue(when, NOW, CHICAGO)).toBe('today 5:00 PM (floating)');
  });
});

describe('dueFromInputs', () => {
  it('returns null without a date', () => {
    expect(dueFromInputs('', '17:00')).toBeNull();
  });

  it('builds a date when the time is empty', () => {
    expect(dueFromInputs('2026-11-06', '')).toEqual({ kind: 'date', date: '2026-11-06' });
  });

  it('builds a zoned value in the device zone when a time is set', () => {
    expect(dueFromInputs('2026-11-06', '17:30')).toEqual({
      kind: 'zoned',
      datetime: '2026-11-06T17:30:00',
      zone: DEVICE_ZONE,
    });
  });
});

describe('dueToInputs', () => {
  it('returns empty inputs for null', () => {
    expect(dueToInputs(null)).toEqual({ date: '', time: '' });
  });

  it('returns an empty time for a date', () => {
    expect(dueToInputs(dateWhen('2026-11-06'))).toEqual({ date: '2026-11-06', time: '' });
  });

  it('splits floating and zoned datetimes', () => {
    expect(dueToInputs(floatingWhen('2026-11-06T17:30:00'))).toEqual({
      date: '2026-11-06',
      time: '17:30',
    });
    const zoned = zonedWhen('2026-11-06T17:30:00', CHICAGO);
    expect(dueToInputs(zoned)).toEqual({ date: '2026-11-06', time: '17:30' });
  });

  it('converts an instant to the device zone', () => {
    const inputs = dueToInputs({ kind: 'instant', timestamp: '2026-11-06T23:00:00Z' });
    expect(inputs.date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect(inputs.time).toMatch(/^\d{2}:\d{2}$/);
  });

  it('round-trips a date and a zoned value', () => {
    expect(dueToInputs(dueFromInputs('2026-11-06', ''))).toEqual({
      date: '2026-11-06',
      time: '',
    });
    expect(dueToInputs(dueFromInputs('2026-11-06', '17:30'))).toEqual({
      date: '2026-11-06',
      time: '17:30',
    });
  });
});

describe('sameDue', () => {
  it('treats two nulls as equal', () => {
    expect(sameDue(null, null)).toBe(true);
  });

  it('treats null and a value as different', () => {
    expect(sameDue(null, dateWhen('2026-11-06'))).toBe(false);
    expect(sameDue(dateWhen('2026-11-06'), null)).toBe(false);
  });

  it('compares matching kinds by their fields', () => {
    expect(sameDue(dateWhen('2026-11-06'), dateWhen('2026-11-06'))).toBe(true);
    expect(sameDue(dateWhen('2026-11-06'), dateWhen('2026-11-07'))).toBe(false);

    const chicago = zonedWhen('2026-11-06T17:00:00', CHICAGO);
    const newYork = zonedWhen('2026-11-06T17:00:00', NEW_YORK);
    expect(sameDue(chicago, newYork)).toBe(false);
  });

  it('treats different kinds as different', () => {
    expect(sameDue(dateWhen('2026-11-06'), floatingWhen('2026-11-06T00:00:00'))).toBe(false);
  });
});
