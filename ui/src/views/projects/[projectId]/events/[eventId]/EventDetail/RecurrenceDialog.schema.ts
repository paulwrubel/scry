import { DateTime } from 'luxon';
import { z } from 'zod';
import type {
  MonthlyMode,
  RecurrenceEnd,
  RecurrenceFrequency,
  YearlyMode,
} from '@/utils/events/recurrence';

export const FREQUENCIES: { value: RecurrenceFrequency; label: string }[] = [
  { value: 'daily', label: 'Daily' },
  { value: 'weekly', label: 'Weekly' },
  { value: 'monthly', label: 'Monthly' },
  { value: 'yearly', label: 'Yearly' },
];

export const UNITS: Record<RecurrenceFrequency, string> = {
  daily: 'day(s)',
  weekly: 'week(s)',
  monthly: 'month(s)',
  yearly: 'year(s)',
};

export const WEEKDAYS = [
  { value: 1, label: 'Mon' },
  { value: 2, label: 'Tue' },
  { value: 3, label: 'Wed' },
  { value: 4, label: 'Thu' },
  { value: 5, label: 'Fri' },
  { value: 6, label: 'Sat' },
  { value: 7, label: 'Sun' },
];

export const ORDINALS = [
  { value: 1, label: 'first' },
  { value: 2, label: 'second' },
  { value: 3, label: 'third' },
  { value: 4, label: 'fourth' },
  { value: -1, label: 'last' },
];

export const MONTHS = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
].map((label, index) => ({ value: index + 1, label }));

export const recurrenceSchema = z
  .object({
    frequency: z.enum(['daily', 'weekly', 'monthly', 'yearly']),
    interval: z.number().int().min(1, 'Use a number of at least 1.'),
    weeklyDays: z.array(z.number()).min(1, 'Choose at least one weekday.'),
    weeklyStart: z.number().int().min(1).max(7),
    monthlyMode: z.enum(['day', 'weekday']),
    monthlyDay: z.number().int().min(1).max(31),
    monthlyOrdinal: z
      .number()
      .int()
      .refine((value) => isOrdinal(value), 'Choose an ordinal.'),
    monthlyWeekday: z.number().int().min(1).max(7),
    yearlyMode: z.enum(['date', 'weekday']),
    yearlyMonth: z.number().int().min(1).max(12),
    yearlyDay: z.number().int().min(1).max(31),
    yearlyOrdinal: z
      .number()
      .int()
      .refine((value) => isOrdinal(value), 'Choose an ordinal.'),
    yearlyWeekday: z.number().int().min(1).max(7),
    endMode: z.enum(['never', 'count', 'until']),
    count: z.number().int().min(1, 'Use at least one occurrence.'),
    until: z.string(),
  })
  .superRefine((value, ctx) => {
    if (value.endMode === 'until' && !DateTime.fromISO(value.until).isValid) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['until'],
        message: 'Choose an end date.',
      });
    }
  });

export function parseFrequency(value: string): RecurrenceFrequency {
  switch (value) {
    case 'daily':
    case 'weekly':
    case 'monthly':
    case 'yearly':
      return value;
    default:
      return 'weekly';
  }
}

export function parseMonthlyMode(value: string): MonthlyMode {
  return value === 'weekday' ? 'weekday' : 'day';
}

export function parseYearlyMode(value: string): YearlyMode {
  return value === 'weekday' ? 'weekday' : 'date';
}

export function parseEnd(value: string): RecurrenceEnd {
  switch (value) {
    case 'count':
    case 'until':
      return value;
    default:
      return 'never';
  }
}

function isOrdinal(value: number): boolean {
  return value === -1 || (value >= 1 && value <= 4);
}
