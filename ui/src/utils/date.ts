import { DateTime } from 'luxon';

/** The UTC timestamp the client writes for `created_at` and `updated_at`. */
export function nowIso(): string {
  return DateTime.now().toUTC().toISO() ?? '';
}

/** Locale-formatted calendar date, e.g. `1/1/2026`. */
export function formatDate(value: string | DateTime): string {
  return toDateTime(value).toLocaleString(DateTime.DATE_SHORT);
}

/** Locale-formatted date and time, e.g. `Jan 1, 2026, 12:00 PM`. */
export function formatDateTime(value: string | DateTime): string {
  return toDateTime(value).toLocaleString(DateTime.DATETIME_MED);
}

function toDateTime(value: string | DateTime): DateTime {
  return typeof value === 'string' ? DateTime.fromISO(value) : value;
}
