import { DateTime } from 'luxon';
import { DEVICE_ZONE } from '@/contract/due';
import { formatRrule, parseRrule } from '@/contract/recur';
import type { RecurrencePattern } from '@/contract/recur';

export type RecurrenceFrequency = 'daily' | 'weekly' | 'monthly' | 'yearly';
export type RecurrenceEnd = 'never' | 'count' | 'until';
export type MonthlyMode = 'day' | 'weekday';
export type YearlyMode = 'date' | 'weekday';

/// The Outlook-style recurrence controls. `wkst` uses Luxon weekdays (1=Mon).
export type RecurrenceDraft = {
  frequency: RecurrenceFrequency;
  interval: number;
  weeklyDays: number[];
  weeklyStart: number;
  monthlyMode: MonthlyMode;
  monthlyDay: number;
  monthlyOrdinal: number;
  monthlyWeekday: number;
  yearlyMode: YearlyMode;
  yearlyMonth: number;
  yearlyDay: number;
  yearlyOrdinal: number;
  yearlyWeekday: number;
  endMode: RecurrenceEnd;
  count: number;
  until: string;
};

const WEEKDAY_LABELS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
const ORDINAL_LABELS: Record<number, string> = {
  1: 'first',
  2: 'second',
  3: 'third',
  4: 'fourth',
  [-1]: 'last',
};

function validStart(startDate: string): DateTime {
  const parsed = DateTime.fromISO(startDate);
  return parsed.isValid ? parsed : DateTime.now();
}

/// Outlook's defaults for a new rule: weekly on the event's start weekday.
export function defaultRecurrenceDraft(startDate: string): RecurrenceDraft {
  const start = validStart(startDate);
  return {
    frequency: 'weekly',
    interval: 1,
    weeklyDays: [start.weekday],
    weeklyStart: 1,
    monthlyMode: 'day',
    monthlyDay: start.day,
    monthlyOrdinal: 1,
    monthlyWeekday: start.weekday,
    yearlyMode: 'date',
    yearlyMonth: start.month,
    yearlyDay: start.day,
    yearlyOrdinal: 1,
    yearlyWeekday: start.weekday,
    endMode: 'never',
    count: 10,
    until: start.plus({ years: 1 }).toISODate() ?? '',
  };
}

/// Map a stored RRULE onto the Outlook controls. Returns null when the rule
/// uses a shape those controls cannot represent, so the editor can stay
/// read-only instead of silently changing its meaning.
export function draftFromRrule(text: string, startDate: string): RecurrenceDraft | null {
  const parsed = parseRrule(text);
  if (parsed.pattern === null) {
    return null;
  }

  const start = validStart(startDate);
  const pattern = parsed.pattern;
  const draft = defaultRecurrenceDraft(startDate);
  draft.interval = pattern.interval;
  draft.weeklyStart = pattern.wkst;

  if (pattern.count !== null) {
    draft.endMode = 'count';
    draft.count = pattern.count;
  } else if (pattern.until !== null) {
    draft.endMode = 'until';
    const until =
      pattern.until.kind === 'date'
        ? pattern.until.date
        : DateTime.fromISO(pattern.until.timestamp, { setZone: true })
            .setZone(DEVICE_ZONE)
            .toISODate();
    if (until === null) {
      return null;
    }
    draft.until = until;
  }

  switch (pattern.freq) {
    case 'daily':
      if (pattern.byDay.length > 0 || pattern.byMonthDay.length > 0 || pattern.byMonth.length > 0) {
        return null;
      }
      draft.frequency = 'daily';
      return draft;
    case 'weekly': {
      if (pattern.byMonthDay.length > 0 || pattern.byMonth.length > 0) {
        return null;
      }
      if (pattern.byDay.some((day) => day.ordinal !== null)) {
        return null;
      }
      draft.frequency = 'weekly';
      const days = pattern.byDay.map((day) => day.weekday);
      draft.weeklyDays = sortedWeekdays(
        days.length === 0 ? [start.weekday] : [...days, start.weekday],
      );
      return draft;
    }
    case 'monthly': {
      if (pattern.byMonth.length > 0) {
        return null;
      }
      draft.frequency = 'monthly';
      if (pattern.byMonthDay.length > 0) {
        const day = single(pattern.byMonthDay);
        if (day === null || day <= 0) {
          return null;
        }
        draft.monthlyMode = 'day';
        draft.monthlyDay = day;
        return draft;
      }
      if (pattern.byDay.length > 0) {
        const day = single(pattern.byDay);
        if (day === null || day.ordinal === null || !isOrdinal(day.ordinal)) {
          return null;
        }
        draft.monthlyMode = 'weekday';
        draft.monthlyOrdinal = day.ordinal;
        draft.monthlyWeekday = day.weekday;
        return draft;
      }
      draft.monthlyMode = 'day';
      draft.monthlyDay = start.day;
      return draft;
    }
    case 'yearly': {
      if (pattern.byMonth.length > 1) {
        return null;
      }
      // A yearly day/weekday without BYMONTH is relative to the whole year,
      // which the month-based Outlook controls cannot represent.
      if (
        pattern.byMonth.length === 0 &&
        (pattern.byMonthDay.length > 0 || pattern.byDay.length > 0)
      ) {
        return null;
      }
      draft.frequency = 'yearly';
      draft.yearlyMonth = pattern.byMonth.at(0) ?? start.month;

      if (pattern.byMonthDay.length > 0) {
        const day = single(pattern.byMonthDay);
        if (day === null || day <= 0) {
          return null;
        }
        draft.yearlyMode = 'date';
        draft.yearlyDay = day;
        return draft;
      }
      if (pattern.byDay.length > 0) {
        const day = single(pattern.byDay);
        if (day === null || day.ordinal === null || !isOrdinal(day.ordinal)) {
          return null;
        }
        draft.yearlyMode = 'weekday';
        draft.yearlyOrdinal = day.ordinal;
        draft.yearlyWeekday = day.weekday;
        return draft;
      }
      draft.yearlyMode = 'date';
      draft.yearlyDay = start.day;
      return draft;
    }
  }
}

/// Build canonical RRULE text from the Outlook controls.
export function rruleFromDraft(draft: RecurrenceDraft): string {
  const pattern: RecurrencePattern = {
    freq: draft.frequency,
    interval: draft.interval,
    byDay: [],
    byMonthDay: [],
    byMonth: [],
    count: draft.endMode === 'count' ? draft.count : null,
    until: draft.endMode === 'until' ? { kind: 'date', date: draft.until } : null,
    wkst: draft.weeklyStart,
  };

  if (draft.frequency === 'weekly') {
    pattern.byDay = sortedWeekdays(draft.weeklyDays).map((weekday) => ({
      ordinal: null,
      weekday,
    }));
  }
  if (draft.frequency === 'monthly') {
    if (draft.monthlyMode === 'day') {
      pattern.byMonthDay = [draft.monthlyDay];
    } else {
      pattern.byDay = [{ ordinal: draft.monthlyOrdinal, weekday: draft.monthlyWeekday }];
    }
  }
  if (draft.frequency === 'yearly') {
    pattern.byMonth = [draft.yearlyMonth];
    if (draft.yearlyMode === 'date') {
      pattern.byMonthDay = [draft.yearlyDay];
    } else {
      pattern.byDay = [{ ordinal: draft.yearlyOrdinal, weekday: draft.yearlyWeekday }];
    }
  }

  return formatRrule(pattern);
}

/// A short line for the Repeat field, e.g. "Every 2 weeks on Mon, Wed, until Nov 30".
export function describeDraft(draft: RecurrenceDraft): string {
  const frequency = describeFrequency(draft);
  if (draft.endMode === 'count') {
    return `${frequency}, for ${draft.count} occurrence${draft.count === 1 ? '' : 's'}`;
  }
  if (draft.endMode === 'until') {
    const until = DateTime.fromISO(draft.until);
    if (until.isValid) {
      return `${frequency}, until ${until.toFormat('LLL d, yyyy')}`;
    }
  }
  return frequency;
}

function describeFrequency(draft: RecurrenceDraft): string {
  const every = draft.interval === 1;

  switch (draft.frequency) {
    case 'daily':
      return every ? 'Every day' : `Every ${draft.interval} days`;
    case 'weekly': {
      const days = sortedWeekdays(draft.weeklyDays)
        .map((weekday) => WEEKDAY_LABELS[weekday - 1])
        .join(', ');
      return every ? `Every week on ${days}` : `Every ${draft.interval} weeks on ${days}`;
    }
    case 'monthly': {
      const prefix = every ? 'Every month' : `Every ${draft.interval} months`;
      if (draft.monthlyMode === 'day') {
        return `Day ${draft.monthlyDay} of ${prefix.toLowerCase()}`;
      }
      const ordinal = ORDINAL_LABELS[draft.monthlyOrdinal] ?? 'first';
      const weekday = WEEKDAY_LABELS[draft.monthlyWeekday - 1];
      return `The ${ordinal} ${weekday} of ${prefix.toLowerCase()}`;
    }
    case 'yearly': {
      const prefix = every ? 'Every year' : `Every ${draft.interval} years`;
      const month = DateTime.fromObject({ month: draft.yearlyMonth }).toFormat('LLLL');
      if (draft.yearlyMode === 'date') {
        return `${prefix} on ${month} ${draft.yearlyDay}`;
      }
      const ordinal = ORDINAL_LABELS[draft.yearlyOrdinal] ?? 'first';
      const weekday = WEEKDAY_LABELS[draft.yearlyWeekday - 1];
      return `The ${ordinal} ${weekday} of ${month} ${every ? 'every year' : `every ${draft.interval} years`}`;
    }
  }
}

function single<T>(values: T[]): T | null {
  return values.length === 1 ? values[0] : null;
}

function isOrdinal(value: number): boolean {
  return value === -1 || (value >= 1 && value <= 4);
}

function sortedWeekdays(values: number[]): number[] {
  return [...new Set(values)].sort((left, right) => left - right);
}
