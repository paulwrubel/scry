import type { DateTime } from 'luxon';
import { sameDue } from '@/contract/due';
import { instantOf } from '@/contract/interval';
import {
  expandRecurrence,
  formatRrule,
  parseRrule,
  type RecurrencePattern,
  type Until,
} from '@/contract/recur';
import type { Event } from '@/generated/Event';
import type { EventOverride } from '@/generated/EventOverride';
import type { Interval } from '@/generated/Interval';
import type { RRule } from '@/generated/RRule';
import type { When } from '@/generated/When';

export type EffectiveOccurrence = {
  interval: Interval;
  title: string;
  description: string | null;
  overridden: boolean;
};

export type SeriesSplit = {
  previous: When;
  occurrencesBefore: number;
  truncated: RRule;
  following: RRule;
  keptOverrides: EventOverride[];
  followingOverrides: EventOverride[];
};

export function overrideFor(event: Event, originalStart: When): EventOverride | undefined {
  return (event.overrides ?? []).find((override) =>
    sameDue(override.original_start, originalStart),
  );
}

export function generatedInterval(
  event: Event,
  originalStart: When,
  zone: string,
): Interval | null {
  if (event.recurrence === null || event.recurrence === undefined) {
    return null;
  }

  const instant: DateTime = instantOf(originalStart, zone);
  let intervals: Interval[];
  try {
    intervals = expandRecurrence(
      event.interval,
      event.recurrence,
      instant.minus({ days: 1 }),
      instant.plus({ days: 1 }),
      zone,
    );
  } catch {
    return null;
  }

  return intervals.find((interval) => sameDue(interval.start, originalStart)) ?? null;
}

export function effectiveOccurrence(
  event: Event,
  originalStart: When,
  zone: string,
): EffectiveOccurrence | null {
  if (event.recurrence === null || event.recurrence === undefined) {
    return null;
  }

  const override = overrideFor(event, originalStart);
  const generated = generatedInterval(event, originalStart, zone);
  const interval = override?.interval ?? generated;
  if (interval === null) {
    return null;
  }

  return {
    interval,
    title: override?.title ?? event.title,
    description: override?.description ?? event.description,
    overridden: override !== undefined,
  };
}

export function splitAt(event: Event, originalStart: When, zone: string): SeriesSplit | null {
  if (event.recurrence === null || event.recurrence === undefined) {
    return null;
  }

  const base: DateTime = instantOf(event.interval.start, zone);
  const target: DateTime = instantOf(originalStart, zone);
  if (target.toMillis() <= base.toMillis()) {
    return null;
  }

  const parsed = parseRrule(event.recurrence.text);
  if (parsed.pattern === null) {
    return null;
  }
  const pattern = parsed.pattern;

  let expanded: Interval[];
  try {
    expanded = expandRecurrence(
      event.interval,
      event.recurrence,
      base.minus({ milliseconds: 1 }),
      target,
      zone,
    );
  } catch {
    return null;
  }

  const kept = expanded.filter(
    (interval) => instantOf(interval.start, zone).toMillis() < target.toMillis(),
  );
  const previous = kept.at(-1)?.start;
  if (previous === undefined) {
    return null;
  }

  // COUNT counts every candidate the rule generates, including starts an
  // EXDATE removes later, so count those before the split separately
  const generatedBefore = countGeneratedBefore(event, event.recurrence.text, base, target, zone);
  if (generatedBefore === null) {
    return null;
  }

  // the pattern's arrays are never mutated, so a shallow clone is enough to
  // give the two halves their own count or until
  const truncatedPattern: RecurrencePattern = { ...pattern };
  const followingPattern: RecurrencePattern = { ...pattern };

  if (pattern.count !== null) {
    if (generatedBefore >= pattern.count) {
      return null;
    }
    truncatedPattern.count = generatedBefore;
    followingPattern.count = pattern.count - generatedBefore;
  } else {
    const until = untilOf(previous, zone);
    if (until === null) {
      return null;
    }
    truncatedPattern.until = until;
  }

  const { kept: keptExdates, following: followingExdates } = partitionAt(
    event.recurrence.exdates ?? [],
    (exdate) => instantOf(exdate, zone),
    target,
  );

  const { kept: keptRdates, following: followingRdates } = partitionAt(
    event.recurrence.rdates ?? [],
    (rdate) => instantOf(rdate, zone),
    target,
  );

  const { kept: keptOverrides, following: followingOverrides } = partitionAt(
    event.overrides ?? [],
    (override) => instantOf(override.original_start, zone),
    target,
  );

  const truncated: RRule = { text: formatRrule(truncatedPattern) };
  if (keptExdates.length > 0) {
    truncated.exdates = keptExdates;
  }
  if (keptRdates.length > 0) {
    truncated.rdates = keptRdates;
  }

  const following: RRule = { text: formatRrule(followingPattern) };
  if (followingExdates.length > 0) {
    following.exdates = followingExdates;
  }
  if (followingRdates.length > 0) {
    following.rdates = followingRdates;
  }

  return {
    previous,
    occurrencesBefore: generatedBefore,
    truncated,
    following,
    keptOverrides,
    followingOverrides,
  };
}

export function truncateAt(
  event: Event,
  originalStart: When,
  zone: string,
): { recurrence: RRule; overrides: EventOverride[] } | null {
  const split = splitAt(event, originalStart, zone);
  if (split === null) {
    return null;
  }
  return { recurrence: split.truncated, overrides: split.keptOverrides };
}

/// Split values at the target instant. A value whose instant cannot be read
/// stays with the truncated side, because it cannot be placed after the split.
function partitionAt<T>(
  values: T[],
  instantOfValue: (value: T) => DateTime,
  target: DateTime,
): { kept: T[]; following: T[] } {
  const kept: T[] = [];
  const following: T[] = [];

  for (const value of values) {
    const instant = instantOfValue(value);
    if (!instant.isValid || instant.toMillis() < target.toMillis()) {
      kept.push(value);
    } else {
      following.push(value);
    }
  }

  return { kept, following };
}

function countGeneratedBefore(
  event: Event,
  text: string,
  from: DateTime,
  to: DateTime,
  zone: string,
): number | null {
  try {
    // no exdates: COUNT is budgeted against generated candidates, not the
    // subset that survives exclusion
    const generated = expandRecurrence(event.interval, { text }, from, to, zone);
    return generated.filter(
      (interval) => instantOf(interval.start, zone).toMillis() < to.toMillis(),
    ).length;
  } catch {
    return null;
  }
}

function untilOf(when: When, zone: string): Until | null {
  switch (when.kind) {
    case 'date':
      return { kind: 'date', date: when.date };
    case 'floating':
      return { kind: 'date', date: when.datetime.slice(0, 10) };
    case 'instant':
      return { kind: 'instant', timestamp: when.timestamp };
    case 'zoned': {
      const iso = instantOf(when, zone).toUTC().toISO();
      return iso === null ? null : { kind: 'instant', timestamp: iso };
    }
  }
}
