import { DateTime, Duration, type DurationLikeObject } from 'luxon';
import type { Interval } from '@/generated/Interval';
import type { RRule } from '@/generated/RRule';
import type { When } from '@/generated/When';
import { instantOf } from '@/contract/interval';
import { parseRrule } from './parse';
import type { RecurrencePattern, Until } from './types';

/// The cap used to stop an endless series.
const MAX_CANDIDATES = 100_000;

export function expandRecurrence(
  interval: Interval,
  rrule: RRule,
  from: DateTime,
  to: DateTime,
  zone: string,
): Interval[] {
  const parsed = parseRrule(rrule.text);
  if (parsed.pattern === null) {
    throw new Error(parsed.error);
  }

  const rule = parsed.pattern;
  const base = civilOf(interval.start);
  const duration = civilDuration(interval.start, interval.end);
  const exdates = rrule.exdates ?? [];
  const rdates = rrule.rdates ?? [];
  const fromMillis = from.toMillis();
  const toMillis = to.toMillis();
  const out: Interval[] = [];
  let generated = 0;
  let examined = 0;

  const emit = (candidate: DateTime): boolean => {
    const start = rebuild(base.kind, candidate);
    const startInstant = instantOf(start, zone);

    if (rule.count !== null && generated >= rule.count) {
      return false;
    }
    if (!untilAllows(rule.until, candidate, startInstant)) {
      return false;
    }
    generated += 1;

    const end = rebuild(base.kind, candidate.plus(duration));
    const occurrence: Interval = { start, end };
    if (startInstant.toMillis() >= toMillis) {
      return false;
    }

    const excluded = exdates.some((exdate) => {
      const instant = instantOf(exdate, zone);
      return instant.isValid && instant.toMillis() === startInstant.toMillis();
    });
    if (
      !excluded &&
      instantOf(occurrence.start, zone).toMillis() < toMillis &&
      fromMillis < instantOf(occurrence.end, zone).toMillis()
    ) {
      out.push(occurrence);
    }

    return true;
  };

  for (const candidate of candidates(base, rule)) {
    examined += 1;
    if (examined > MAX_CANDIDATES) {
      throw new Error(`recurrence exceeded ${MAX_CANDIDATES} candidate occurrences`);
    }
    if (!emit(candidate)) {
      break;
    }
  }

  // an RDATE names only a start, so its end is the series' civil duration
  // applied to that start with the series' own kind; COUNT and UNTIL do not
  // apply, which also keeps `generated` untouched
  for (const rdate of rdates) {
    const candidate = civilOf(rdate).civil;
    const start = rebuild(base.kind, candidate);
    const startMillis = instantOf(start, zone).toMillis();

    const alreadyEmitted = out.some(
      (existing) => instantOf(existing.start, zone).toMillis() === startMillis,
    );
    const excluded = exdates.some((exdate) => {
      const instant = instantOf(exdate, zone);
      return instant.isValid && instant.toMillis() === startMillis;
    });
    if (alreadyEmitted || excluded || startMillis >= toMillis) {
      continue;
    }

    const end = rebuild(base.kind, candidate.plus(duration));
    const occurrence: Interval = { start, end };
    if (fromMillis < instantOf(occurrence.end, zone).toMillis()) {
      out.push(occurrence);
    }
  }

  // an added start can fall between the generated ones, so restore start order
  out.sort(
    (left, right) =>
      instantOf(left.start, zone).toMillis() - instantOf(right.start, zone).toMillis(),
  );

  return out;
}

type Kind =
  { kind: 'date' } | { kind: 'floating' } | { kind: 'instant' } | { kind: 'zoned'; zone: string };

type CivilWhen = { kind: Kind; civil: DateTime };

/// Read a stored value on a zone-free civil calendar. Luxon has no bare civil
/// type, so UTC stands in for "no zone"; only the components are ever used.
function civilOf(when: When): CivilWhen {
  switch (when.kind) {
    case 'date':
      return {
        kind: { kind: 'date' },
        civil: DateTime.fromISO(when.date, { zone: 'UTC' }).startOf('day'),
      };
    case 'floating':
      return {
        kind: { kind: 'floating' },
        civil: DateTime.fromISO(when.datetime, { zone: 'UTC' }),
      };
    case 'instant':
      return {
        kind: { kind: 'instant' },
        civil: DateTime.fromISO(when.timestamp, { setZone: true }).toUTC(),
      };
    case 'zoned':
      return {
        kind: { kind: 'zoned', zone: when.zone },
        civil: DateTime.fromISO(when.datetime, { zone: 'UTC' }),
      };
  }
}

function rebuild(kind: Kind, civil: DateTime): When {
  const datetime = civil.toFormat("yyyy-LL-dd'T'HH:mm:ss");
  switch (kind.kind) {
    case 'date':
      return { kind: 'date', date: civil.toISODate() ?? '' };
    case 'floating':
      return { kind: 'floating', datetime };
    case 'instant':
      return {
        kind: 'instant',
        timestamp: civil.toFormat("yyyy-LL-dd'T'HH:mm:ss'Z'"),
      };
    case 'zoned':
      return { kind: 'zoned', datetime, zone: kind.zone };
  }
}

/// The civil difference between the interval ends. Both ends sit in UTC here,
/// so DST cannot change the number of nominal days or hours.
function civilDuration(start: When, end: When): DurationLikeObject {
  return Duration.fromMillis(civilOf(end).civil.toMillis() - civilOf(start).civil.toMillis())
    .shiftTo('days', 'hours', 'minutes', 'seconds', 'milliseconds')
    .toObject();
}

function daysSince(from: number, to: number): number {
  return (((to - from) % 7) + 7) % 7;
}

function startOfWeek(date: DateTime, wkst: number): DateTime {
  return date.minus({ days: daysSince(wkst, date.weekday) }).startOf('day');
}

function sortedUnique(values: number[]): number[] {
  return [...new Set(values)].sort((left, right) => left - right);
}

function sortedUniqueDates(values: DateTime[]): DateTime[] {
  const seen = new Set<number>();
  return [...values]
    .sort((left, right) => left.toMillis() - right.toMillis())
    .filter((value) => {
      const key = value.toMillis();
      if (seen.has(key)) {
        return false;
      }
      seen.add(key);
      return true;
    });
}

/// The dates one month contributes, chosen by BYMONTHDAY, BYDAY (including
/// ordinals), or the base day of month when neither is present.
function daySelection(
  rule: RecurrencePattern,
  year: number,
  month: number,
  baseDay: number,
): DateTime[] {
  const first = DateTime.fromObject({ year, month, day: 1 }, { zone: 'UTC' });
  if (!first.isValid) {
    return [];
  }
  const daysInMonth = first.daysInMonth ?? 0;
  const days: DateTime[] = [];

  if (rule.byMonthDay.length > 0) {
    for (const day of rule.byMonthDay) {
      const resolved = day > 0 ? day : daysInMonth + day + 1;
      if (resolved < 1 || resolved > daysInMonth) {
        continue;
      }
      days.push(first.set({ day: resolved }));
    }
  } else if (rule.byDay.length > 0) {
    for (const byDay of rule.byDay) {
      if (byDay.ordinal !== null) {
        const date = nthWeekdayOfMonth(first, byDay.ordinal, byDay.weekday);
        if (date !== null) {
          days.push(date);
        }
        continue;
      }
      const offset = daysSince(first.weekday, byDay.weekday);
      let current = first.plus({ days: offset });
      while (current.isValid && current.month === month) {
        days.push(current);
        current = current.plus({ days: 7 });
      }
    }
  } else {
    if (baseDay <= daysInMonth) {
      days.push(first.set({ day: baseDay }));
    }
  }

  return sortedUniqueDates(days);
}

function nthWeekdayOfMonth(first: DateTime, ordinal: number, weekday: number): DateTime | null {
  const daysInMonth = first.daysInMonth ?? 0;
  if (ordinal > 0) {
    const day = 1 + daysSince(first.weekday, weekday) + 7 * (ordinal - 1);
    return day <= daysInMonth ? first.set({ day }) : null;
  }

  const last = first.set({ day: daysInMonth });
  const day = daysInMonth - daysSince(weekday, last.weekday) + 7 * (ordinal + 1);
  return day >= 1 ? first.set({ day }) : null;
}

/// The nth weekday of a whole year, used when YEARLY has an ordinal BYDAY and
/// no BYMONTH.
function nthWeekdayOfYear(year: number, ordinal: number, weekday: number): DateTime | null {
  if (ordinal === 0) {
    return null;
  }

  const first = DateTime.fromObject({ year, month: 1, day: 1 }, { zone: 'UTC' });
  const last = DateTime.fromObject({ year, month: 12, day: 31 }, { zone: 'UTC' });
  if (!first.isValid || !last.isValid) {
    return null;
  }

  const date =
    ordinal > 0
      ? first.plus({ days: daysSince(first.weekday, weekday) + 7 * (ordinal - 1) })
      : last.plus({ days: -daysSince(weekday, last.weekday) + 7 * (ordinal + 1) });

  return date.isValid && date.year === year ? date : null;
}

function untilAllows(until: Until | null, datetime: DateTime, startInstant: DateTime): boolean {
  if (until === null) {
    return true;
  }
  if (until.kind === 'date') {
    const date = datetime.toISODate();
    return date !== null && date <= until.date;
  }
  return startInstant.toMillis() <= DateTime.fromISO(until.timestamp, { setZone: true }).toMillis();
}

/// Yield the civil candidate starts for a series, base first. The caller applies
/// COUNT, UNTIL, EXDATE, and the window, so these functions only choose dates.
function* candidates(base: CivilWhen, rule: RecurrencePattern): Generator<DateTime> {
  yield base.civil;
  switch (rule.freq) {
    case 'daily':
      yield* dailyCandidates(base, rule);
      break;
    case 'weekly':
      yield* weeklyCandidates(base, rule);
      break;
    case 'monthly':
      yield* monthlyCandidates(base, rule);
      break;
    case 'yearly':
      yield* yearlyCandidates(base, rule);
      break;
  }
}

/// Step whole days and keep the dates that satisfy the BY filters.
function* dailyCandidates(base: CivilWhen, rule: RecurrencePattern): Generator<DateTime> {
  const first = base.civil.startOf('day');
  for (let index = 1; ; index += 1) {
    const date = first.plus({ days: index * rule.interval });
    if (!date.isValid) {
      return;
    }
    if (dayMatches(rule, date)) {
      yield atBaseTime(base.civil, date);
    }
  }
}

/// Step whole weeks from the WKST-aligned week containing the base date.
function* weeklyCandidates(base: CivilWhen, rule: RecurrencePattern): Generator<DateTime> {
  const baseDate = base.civil.startOf('day');
  const weekStart = startOfWeek(baseDate, rule.wkst);
  const weekdays =
    rule.byDay.length === 0 ? [baseDate.weekday] : rule.byDay.map((day) => day.weekday);
  const offsets = sortedUnique(weekdays.map((weekday) => daysSince(rule.wkst, weekday)));

  for (let index = 0; ; index += 1) {
    const start = weekStart.plus({ days: index * rule.interval * 7 });
    if (!start.isValid) {
      return;
    }

    for (const offset of offsets) {
      const date = start.plus({ days: offset });
      if (!date.isValid) {
        continue;
      }
      const candidate = atBaseTime(base.civil, date);
      if (candidate.toMillis() <= base.civil.toMillis() || !dayMatches(rule, date)) {
        continue;
      }
      yield candidate;
    }
  }
}

/// Step whole months and emit the selected day-of-month set.
function* monthlyCandidates(base: CivilWhen, rule: RecurrencePattern): Generator<DateTime> {
  const first = base.civil.startOf('month');
  const baseDay = base.civil.day;

  for (let index = 0; ; index += 1) {
    const monthStart = first.plus({ months: index * rule.interval });
    if (!monthStart.isValid) {
      return;
    }

    for (const date of daySelection(rule, monthStart.year, monthStart.month, baseDay)) {
      const candidate = atBaseTime(base.civil, date);
      if (candidate.toMillis() <= base.civil.toMillis()) {
        continue;
      }
      if (rule.byMonth.length > 0 && !rule.byMonth.includes(date.month)) {
        continue;
      }
      yield candidate;
    }
  }
}

/// Step whole years, choosing months from BYMONTH, the BY expansions, or the
/// base month, and handle ordinal BYDAY across the whole year.
function* yearlyCandidates(base: CivilWhen, rule: RecurrencePattern): Generator<DateTime> {
  const baseDate = base.civil.startOf('day');
  const baseDay = base.civil.day;
  const baseYear = base.civil.year;
  const ordinalOnly =
    rule.byMonth.length === 0 &&
    rule.byMonthDay.length === 0 &&
    rule.byDay.length > 0 &&
    rule.byDay.every((day) => day.ordinal !== null);

  for (let index = 0; ; index += 1) {
    const year = baseYear + index * rule.interval;
    if (year < -9999 || year > 9999) {
      return;
    }

    let dates: DateTime[];
    if (ordinalOnly) {
      dates = rule.byDay.flatMap((day) => {
        if (day.ordinal === null) {
          return [];
        }
        const date = nthWeekdayOfYear(year, day.ordinal, day.weekday);
        return date === null ? [] : [date];
      });
    } else {
      let months: number[];
      if (rule.byMonth.length > 0) {
        months = sortedUnique(rule.byMonth);
      } else if (rule.byMonthDay.length > 0 || rule.byDay.length > 0) {
        months = Array.from({ length: 12 }, (_, month) => month + 1);
      } else {
        months = [baseDate.month];
      }
      dates = months.flatMap((month) => daySelection(rule, year, month, baseDay));
    }

    for (const date of sortedUniqueDates(dates)) {
      const candidate = atBaseTime(base.civil, date);
      if (candidate.toMillis() <= base.civil.toMillis()) {
        continue;
      }
      yield candidate;
    }
  }
}

/// Put a generated date at the base start's time of day.
function atBaseTime(base: DateTime, date: DateTime): DateTime {
  return date.set({
    hour: base.hour,
    minute: base.minute,
    second: base.second,
    millisecond: base.millisecond,
  });
}

/// Apply BYMONTH, BYMONTHDAY, and BYDAY as filters, which is their role for a
/// DAILY rule.
function dayMatches(rule: RecurrencePattern, date: DateTime): boolean {
  if (rule.byMonth.length > 0 && !rule.byMonth.includes(date.month)) {
    return false;
  }
  if (rule.byMonthDay.length > 0) {
    const daysInMonth = date.daysInMonth ?? 0;
    const matched = rule.byMonthDay.some((day) => {
      const resolved = day > 0 ? day : daysInMonth + day + 1;
      return date.day === resolved;
    });
    if (!matched) {
      return false;
    }
  }
  return rule.byDay.length === 0 || rule.byDay.some((day) => day.weekday === date.weekday);
}
