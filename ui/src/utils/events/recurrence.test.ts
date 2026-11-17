import { describe, expect, it } from 'vitest';
import {
  defaultRecurrenceDraft,
  describeDraft,
  draftFromRrule,
  rruleFromDraft,
} from './recurrence';

const START = '2026-11-16';

describe('defaultRecurrenceDraft', () => {
  it('starts weekly on the event weekday', () => {
    const draft = defaultRecurrenceDraft(START);

    expect(draft).toMatchObject({
      frequency: 'weekly',
      interval: 1,
      weeklyDays: [1],
      weeklyStart: 1,
      endMode: 'never',
      monthlyDay: 16,
      yearlyMonth: 11,
      yearlyDay: 16,
    });
  });
});

describe('draftFromRrule', () => {
  it('reads a daily rule with a count', () => {
    expect(draftFromRrule('FREQ=DAILY;INTERVAL=2;COUNT=5', START)).toMatchObject({
      frequency: 'daily',
      interval: 2,
      endMode: 'count',
      count: 5,
    });
  });

  it('reads a weekly rule with days and week start', () => {
    expect(draftFromRrule('FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;WKST=SU', START)).toMatchObject({
      frequency: 'weekly',
      interval: 2,
      weeklyDays: [1, 3],
      weeklyStart: 7,
    });
  });

  it('adds the start weekday to a weekly rule that omits it', () => {
    expect(draftFromRrule('FREQ=WEEKLY;BYDAY=WE', START)?.weeklyDays).toEqual([1, 3]);
  });

  it('reads a monthly day-of-month rule', () => {
    expect(draftFromRrule('FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=15', START)).toMatchObject({
      frequency: 'monthly',
      interval: 3,
      monthlyMode: 'day',
      monthlyDay: 15,
    });
  });

  it('reads a monthly nth-weekday rule', () => {
    expect(draftFromRrule('FREQ=MONTHLY;BYDAY=-1FR', START)).toMatchObject({
      frequency: 'monthly',
      monthlyMode: 'weekday',
      monthlyOrdinal: -1,
      monthlyWeekday: 5,
    });
  });

  it('reads a yearly date rule', () => {
    expect(draftFromRrule('FREQ=YEARLY;BYMONTH=11;BYMONTHDAY=16', START)).toMatchObject({
      frequency: 'yearly',
      yearlyMode: 'date',
      yearlyMonth: 11,
      yearlyDay: 16,
    });
  });

  it('reads a yearly nth-weekday rule', () => {
    expect(draftFromRrule('FREQ=YEARLY;BYMONTH=11;BYDAY=1MO', START)).toMatchObject({
      frequency: 'yearly',
      yearlyMode: 'weekday',
      yearlyMonth: 11,
      yearlyOrdinal: 1,
      yearlyWeekday: 1,
    });
  });

  it('reads an instant until into a local date', () => {
    expect(draftFromRrule('FREQ=DAILY;UNTIL=20261130T235900Z', START)).toMatchObject({
      endMode: 'until',
      until: '2026-11-30',
    });
  });

  it.each([
    'FREQ=MONTHLY;BYMONTHDAY=-1',
    'FREQ=YEARLY;BYMONTH=1,2',
    'FREQ=DAILY;BYDAY=MO',
    'FREQ=MONTHLY;BYDAY=MO',
    'FREQ=YEARLY;BYDAY=MO',
    'FREQ=YEARLY;BYDAY=-1SU',
    'FREQ=YEARLY;BYMONTHDAY=16',
  ])('returns null for a rule outside the Outlook controls: %s', (text) => {
    expect(draftFromRrule(text, START)).toBeNull();
  });

  it('returns null for an invalid rule', () => {
    expect(draftFromRrule('FREQ=NOPE', START)).toBeNull();
  });
});

describe('rruleFromDraft', () => {
  it('builds a daily interval rule', () => {
    const draft = { ...defaultRecurrenceDraft(START), frequency: 'daily' as const, interval: 3 };
    expect(rruleFromDraft(draft)).toBe('FREQ=DAILY;INTERVAL=3');
  });

  it('builds a weekly rule with days and week start', () => {
    const draft = {
      ...defaultRecurrenceDraft(START),
      interval: 2,
      weeklyDays: [1, 3],
      weeklyStart: 7,
    };
    expect(rruleFromDraft(draft)).toBe('FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;WKST=SU');
  });

  it('builds a monthly day rule', () => {
    const draft = {
      ...defaultRecurrenceDraft(START),
      frequency: 'monthly' as const,
      interval: 3,
      monthlyMode: 'day' as const,
      monthlyDay: 15,
    };
    expect(rruleFromDraft(draft)).toBe('FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=15');
  });

  it('builds a monthly last-Friday rule', () => {
    const draft = {
      ...defaultRecurrenceDraft(START),
      frequency: 'monthly' as const,
      monthlyMode: 'weekday' as const,
      monthlyOrdinal: -1,
      monthlyWeekday: 5,
    };
    expect(rruleFromDraft(draft)).toBe('FREQ=MONTHLY;BYDAY=-1FR');
  });

  it('builds a yearly date rule', () => {
    const draft = {
      ...defaultRecurrenceDraft(START),
      frequency: 'yearly' as const,
      yearlyMode: 'date' as const,
      yearlyMonth: 11,
      yearlyDay: 16,
    };
    expect(rruleFromDraft(draft)).toBe('FREQ=YEARLY;BYMONTH=11;BYMONTHDAY=16');
  });

  it('builds a yearly nth-weekday rule', () => {
    const draft = {
      ...defaultRecurrenceDraft(START),
      frequency: 'yearly' as const,
      yearlyMode: 'weekday' as const,
      yearlyMonth: 11,
      yearlyOrdinal: 1,
      yearlyWeekday: 1,
    };
    expect(rruleFromDraft(draft)).toBe('FREQ=YEARLY;BYMONTH=11;BYDAY=1MO');
  });

  it('builds both end modes', () => {
    const counted = {
      ...defaultRecurrenceDraft(START),
      endMode: 'count' as const,
      count: 4,
    };
    expect(rruleFromDraft(counted)).toBe('FREQ=WEEKLY;BYDAY=MO;COUNT=4');

    const until = {
      ...defaultRecurrenceDraft(START),
      endMode: 'until' as const,
      until: '2026-12-31',
    };
    expect(rruleFromDraft(until)).toBe('FREQ=WEEKLY;BYDAY=MO;UNTIL=20261231');
  });
});

describe('round trips', () => {
  it.each([
    'FREQ=DAILY;INTERVAL=2;COUNT=5',
    'FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;WKST=SU',
    'FREQ=MONTHLY;BYMONTHDAY=15',
    'FREQ=MONTHLY;BYDAY=-1FR',
    'FREQ=YEARLY;BYMONTH=11;BYMONTHDAY=16',
    'FREQ=YEARLY;BYMONTH=11;BYDAY=1MO',
    'FREQ=DAILY;UNTIL=20261231',
  ])('preserves the supported meaning of %s', (text) => {
    const draft = draftFromRrule(text, START);
    expect(draft).not.toBeNull();
    if (draft === null) {
      return;
    }
    expect(draftFromRrule(rruleFromDraft(draft), START)).toEqual(draft);
  });
});

describe('describeDraft', () => {
  it('describes weekly days', () => {
    const draft = {
      ...defaultRecurrenceDraft(START),
      interval: 2,
      weeklyDays: [1, 3],
    };
    expect(describeDraft(draft)).toBe('Every 2 weeks on Mon, Wed');
  });

  it('describes a monthly nth weekday', () => {
    const draft = {
      ...defaultRecurrenceDraft(START),
      frequency: 'monthly' as const,
      monthlyMode: 'weekday' as const,
      monthlyOrdinal: -1,
      monthlyWeekday: 5,
    };
    expect(describeDraft(draft)).toBe('The last Fri of every month');
  });

  it('describes a yearly date with a count', () => {
    const draft = {
      ...defaultRecurrenceDraft(START),
      frequency: 'yearly' as const,
      yearlyMode: 'date' as const,
      yearlyMonth: 11,
      yearlyDay: 16,
      endMode: 'count' as const,
      count: 3,
    };
    expect(describeDraft(draft)).toBe('Every year on November 16, for 3 occurrences');
  });

  it('describes an until date', () => {
    const draft = {
      ...defaultRecurrenceDraft(START),
      endMode: 'until' as const,
      until: '2026-12-31',
    };
    expect(describeDraft(draft)).toBe('Every week on Mon, until Dec 31, 2026');
  });
});
