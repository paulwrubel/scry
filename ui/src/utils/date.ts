/** Locale-formatted calendar date, e.g. `1/1/2026`. */
export function formatDate(value: string | Date): string {
  return new Date(value).toLocaleDateString();
}

/** Locale-formatted date and time, e.g. `Jan 1, 2026, 12:00 PM`. */
export function formatDateTime(value: string | Date): string {
  return new Date(value).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
}
