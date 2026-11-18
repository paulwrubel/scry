import { DateTime } from 'luxon';
import { DEVICE_ZONE } from '@/contract/due';
import { defaultEndInputs, intervalToInputs } from '@/contract/interval';
import type { Event } from '@/generated/Event';
import type { Interval } from '@/generated/Interval';
import type { RRule } from '@/generated/RRule';
import type { When } from '@/generated/When';
import type { EventFieldPatch } from '@/utils/events/operations';
import { effectiveOccurrence } from '@/utils/events/overrides';

export type ApplyTo = 'this' | 'following' | 'all';

export type ApplyToOption = { value: ApplyTo; label: string };

export const APPLY_TO_OPTIONS: ApplyToOption[] = [
  { value: 'this', label: 'This event' },
  { value: 'following', label: 'This and following' },
  { value: 'all', label: 'All events' },
];

export type EventFormValues = {
  title: string;
  description: string;
  startDate: string;
  startTime: string;
  endDate: string;
  endTime: string;
  recurrence: string;
  applyTo: ApplyTo;
};

/// The fields a save submits for the current values. Existing EXDATEs are kept
/// when the rule text is edited.
export function buildEventPatch(
  event: Event | null,
  value: EventFormValues,
  interval: Interval,
): EventFieldPatch {
  const text = value.recurrence.trim();
  const exdates = event?.recurrence?.exdates;
  const recurrence: RRule | null =
    text.length === 0
      ? null
      : {
          text,
          ...(exdates === undefined || exdates.length === 0 ? {} : { exdates }),
        };

  return {
    title: value.title,
    description: value.description,
    interval,
    recurrence,
  };
}

/// The values the form should hold for a mode: the series for "all", and the
/// effective occurrence for the two occurrence-scoped modes.
export function valuesForMode(
  event: Event | null,
  defaultDate: string | null,
  occurrence: When | null,
  mode: ApplyTo,
): EventFormValues {
  const base = initialValues(event, defaultDate, mode === 'all' ? null : occurrence);
  return { ...base, applyTo: mode };
}

/// Whether the fields the mode actually submits differ from its baseline.
/// The recurrence only counts in "all" mode, where it is editable.
export function formHasChanges(
  values: EventFormValues,
  baseline: EventFormValues,
  mode: ApplyTo,
): boolean {
  return (
    values.title !== baseline.title ||
    values.description !== baseline.description ||
    values.startDate !== baseline.startDate ||
    values.startTime !== baseline.startTime ||
    values.endDate !== baseline.endDate ||
    values.endTime !== baseline.endTime ||
    (mode === 'all' && values.recurrence !== baseline.recurrence)
  );
}

export function initialValues(
  event: Event | null,
  defaultDate: string | null,
  occurrence: When | null,
): EventFormValues {
  if (event !== null) {
    const selected =
      occurrence === null ? null : effectiveOccurrence(event, occurrence, DEVICE_ZONE);
    const interval = selected === null ? event.interval : selected.interval;
    const inputs = intervalToInputs(interval);
    const applyTo: ApplyTo = selected === null ? 'all' : 'this';

    return {
      title: selected === null ? event.title : selected.title,
      description: (selected === null ? event.description : selected.description) ?? '',
      startDate: inputs.startDate,
      startTime: inputs.startTime,
      endDate: inputs.endDate,
      endTime: inputs.endTime,
      recurrence: event.recurrence?.text ?? '',
      applyTo,
    };
  }

  const startDate = defaultDate ?? DateTime.now().toISODate() ?? '';
  const applyTo: ApplyTo = 'all';
  return {
    title: '',
    description: '',
    startDate,
    startTime: '',
    endDate: defaultEndInputs(startDate, '').endDate,
    endTime: '',
    recurrence: '',
    applyTo,
  };
}
