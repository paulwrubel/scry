/// A repeat cadence.
export type Frequency = 'daily' | 'weekly' | 'monthly' | 'yearly';

/// A weekday plus an optional ordinal; Luxon uses 1=Monday through 7=Sunday.
export type ByDay = { ordinal: number | null; weekday: number };

export type Until = { kind: 'date'; date: string } | { kind: 'instant'; timestamp: string };

/// A parsed RRULE. The event stores the text; this shape exists while a window
/// is read or a rule is edited.
export type RecurrencePattern = {
  freq: Frequency;
  interval: number;
  byDay: ByDay[];
  byMonthDay: number[];
  byMonth: number[];
  count: number | null;
  until: Until | null;
  wkst: number;
};

export type ParseResult =
  { pattern: RecurrencePattern; error: null } | { pattern: null; error: string };

export const FREQUENCY_CODES: Record<Frequency, string> = {
  daily: 'DAILY',
  weekly: 'WEEKLY',
  monthly: 'MONTHLY',
  yearly: 'YEARLY',
};

/// Indexed by Luxon weekday minus one.
export const WEEKDAY_CODES = ['MO', 'TU', 'WE', 'TH', 'FR', 'SA', 'SU'];
