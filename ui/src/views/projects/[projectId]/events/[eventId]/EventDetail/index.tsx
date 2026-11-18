import { useSelector } from '@tanstack/react-form';
import { Alert, Button, Card, Label, TextInput } from 'flowbite-react';
import { useState } from 'react';
import { z } from 'zod';
import { BackButton } from '@/components/BackButton';
import { fieldErrorMessage } from '@/components/form-controls/fieldErrors';
import { DEVICE_ZONE, sameDue } from '@/contract/due';
import {
  defaultEndInputs,
  intervalFromInputs,
  intervalToInputs,
  shiftEndInputs,
} from '@/contract/interval';
import { parseRrule } from '@/contract/recur';
import type { Event } from '@/generated/Event';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { When } from '@/generated/When';
import { useAppForm } from '@/hooks/useAppForm';
import { useEnqueue } from '@/hooks/useEnqueue';
import { eventSpan } from '@/utils/events/calendar';
import {
  createEventOperation,
  deleteEventOperation,
  deleteOccurrenceOperation,
  setOccurrenceOverrideOperation,
  splitEventOperations,
  truncateSeriesOperation,
  updateEventOperation,
} from '@/utils/events/operations';
import { effectiveOccurrence } from '@/utils/events/overrides';
import { DeleteEventModal } from './DeleteEventModal';
import type { DeleteEventMode } from './DeleteEventModal';
import { OccurrenceModeField } from './OccurrenceModeField';
import { RecurrenceField } from './RecurrenceField';
import { buildEventPatch, formHasChanges, initialValues, valuesForMode } from './values';

const INPUT_CLASS =
  'border-line-strong bg-surface text-zinc-200 focus:border-line-hover w-full rounded-lg border p-2 text-sm focus:outline-none';

const schema = z
  .object({
    title: z.string().trim().min(1, 'An event needs a title.'),
    description: z.string(),
    startDate: z.string(),
    startTime: z.string(),
    endDate: z.string(),
    endTime: z.string(),
    recurrence: z.string(),
    applyTo: z.enum(['this', 'following', 'all']),
  })
  .superRefine((value, ctx) => {
    const result = intervalFromInputs(value);
    if (result.interval === null) {
      ctx.addIssue({ code: z.ZodIssueCode.custom, path: ['startDate'], message: result.error });
    }

    const recurrence = parseRrule(value.recurrence);
    if (value.recurrence.trim().length > 0 && recurrence.error !== null) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['recurrence'],
        message: recurrence.error,
      });
    }
  });

export type EventDetailProps = {
  project: ProjectSnapshot;
  event: Event | null;
  defaultDate: string | null;
  occurrence?: When | null;
  onSaved: (eventId: string) => void;
  onClose: () => void;
};

export function EventDetail(props: EventDetailProps) {
  const { project, event, defaultDate, occurrence = null, onSaved, onClose } = props;

  const [pendingDelete, setPendingDelete] = useState<DeleteEventMode | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [endTouched, setEndTouched] = useState(false);

  const enqueue = useEnqueue();

  const selected =
    event !== null && occurrence !== null
      ? effectiveOccurrence(event, occurrence, DEVICE_ZONE)
      : null;

  const form = useAppForm({
    defaultValues: initialValues(event, defaultDate, occurrence),
    validators: { onChange: schema, onMount: schema },
    onSubmit: async ({ value }) => {
      const result = intervalFromInputs(value);
      if (result.interval === null) {
        return;
      }

      if (selected !== null && event !== null && occurrence !== null && value.applyTo === 'this') {
        const operation = setOccurrenceOverrideOperation(event, occurrence, {
          title: value.title,
          description: value.description,
          interval: result.interval,
        });
        setError(null);
        try {
          if (operation !== null) {
            await enqueue(operation);
          }
        } catch {
          setError('Could not save this event.');
        }
        return;
      }

      if (
        selected !== null &&
        event !== null &&
        occurrence !== null &&
        value.applyTo === 'following'
      ) {
        const operations = splitEventOperations(event, occurrence, {
          title: value.title,
          description: value.description,
          interval: result.interval,
        });
        if (operations === null) {
          setError('Could not split this series.');
          return;
        }
        setError(null);
        try {
          await enqueue(operations.update);
          await enqueue(operations.create);
          onSaved(operations.followingId);
        } catch {
          setError('Could not save this event.');
        }
        return;
      }

      const patch = buildEventPatch(event, value, result.interval);

      setError(null);
      try {
        if (event === null) {
          const created = createEventOperation(project, patch);
          if (created === null) {
            return;
          }
          await enqueue(created.operation);
          onSaved(created.id);
          return;
        }

        const operation = updateEventOperation(event, patch);
        if (operation !== null) {
          await enqueue(operation);
        }
      } catch {
        setError('Could not save this event.');
      }
    },
  });

  const applyTo = useSelector(form.store, (state) => state.values.applyTo);
  const values = useSelector(form.store, (state) => state.values);
  const startDateValue = useSelector(form.store, (state) => state.values.startDate);
  const isSubmitting = useSelector(form.store, (state) => state.isSubmitting);
  const baseline = valuesForMode(event, defaultDate, occurrence, applyTo);
  const hasChanges = formHasChanges(values, baseline, applyTo);

  async function handleDelete() {
    if (event === null) {
      return;
    }
    try {
      await enqueue(deleteEventOperation(event));
      setPendingDelete(null);
      onClose();
    } catch {
      setError('Could not delete this event.');
    }
  }

  async function handleDeleteOccurrence() {
    if (event === null || occurrence === null) {
      return;
    }

    const operation = deleteOccurrenceOperation(event, occurrence);

    try {
      if (operation !== null) {
        await enqueue(operation);
      }
      setPendingDelete(null);
      onClose();
    } catch {
      setError('Could not delete this occurrence.');
    }
  }

  async function handleDeleteFollowing() {
    if (event === null || occurrence === null) {
      return;
    }

    const operation = truncateSeriesOperation(event, occurrence);
    if (operation === null) {
      // removing from the first occurrence leaves nothing, so delete the series
      if (sameDue(occurrence, event.interval.start)) {
        try {
          await enqueue(deleteEventOperation(event));
          setPendingDelete(null);
          onClose();
        } catch {
          setError('Could not delete this event.');
        }
        return;
      }
      setError('Could not delete the following events.');
      return;
    }

    try {
      await enqueue(operation);
      setPendingDelete(null);
      onClose();
    } catch {
      setError('Could not delete this occurrence.');
    }
  }

  // a single-day event's end is really a duration, so it moves with the start;
  // a multi-day end is a committed date and stays where the user put it
  function followStart(
    startDate: string,
    startTime: string,
    previousDate: string,
    previousTime: string,
  ) {
    if (event === null) {
      if (!endTouched) {
        setEnd(defaultEndInputs(startDate, startTime));
      }
      return;
    }

    const shown = selected?.interval ?? event.interval;
    const span = eventSpan({ ...event, interval: shown }, DEVICE_ZONE);
    if (!span.first.hasSame(span.last, 'day')) {
      return;
    }

    const shifted = shiftEndInputs(
      { endDate: form.getFieldValue('endDate'), endTime: form.getFieldValue('endTime') },
      { startDate: previousDate, startTime: previousTime },
      { startDate, startTime },
    );
    setEnd(shifted ?? defaultEndInputs(startDate, startTime));
  }

  function setEnd(end: { endDate: string; endTime: string }) {
    form.setFieldValue('endDate', end.endDate);
    form.setFieldValue('endTime', end.endTime);
  }

  const showingOccurrence = selected !== null;

  return (
    <>
      <div className="flex min-h-full flex-col gap-4 p-6">
        <div className="flex flex-col gap-3">
          <BackButton label="Back to calendar" onClick={onClose} />
          <div className="flex min-w-0 flex-col gap-1">
            <h1 className="min-w-0 text-2xl font-semibold wrap-break-word">
              {event === null ? 'New event' : (selected?.title ?? event.title)}
            </h1>
            <div className="flex min-w-0 flex-wrap items-baseline justify-between gap-2">
              <p className="text-muted text-sm">{project.name}</p>
              {event === null ? null : (
                <span className="text-muted shrink-0 font-mono text-sm">
                  {event.id.slice(0, 8)}
                </span>
              )}
            </div>
          </div>
        </div>

        {error === null ? null : <Alert color="failure">{error}</Alert>}

        {showingOccurrence ? (
          <form.AppField name="applyTo">
            {(field) => (
              <OccurrenceModeField
                value={field.state.value}
                onChange={(value) => {
                  field.handleChange(value);

                  const target = value === 'all' ? event?.interval : selected?.interval;
                  if (target === undefined) {
                    return;
                  }

                  const inputs = intervalToInputs(target);
                  form.setFieldValue('startDate', inputs.startDate);
                  form.setFieldValue('startTime', inputs.startTime);
                  form.setFieldValue('endDate', inputs.endDate);
                  form.setFieldValue('endTime', inputs.endTime);
                }}
              />
            )}
          </form.AppField>
        ) : null}

        <Card>
          <form
            className="flex flex-col gap-4"
            onSubmit={(submitEvent) => {
              submitEvent.preventDefault();
              void form.handleSubmit();
            }}
          >
            <form.AppField name="title">
              {(field) => {
                const titleError =
                  field.state.meta.isTouched || form.state.isSubmitted
                    ? fieldErrorMessage(field.state.meta.errors)
                    : null;

                return (
                  <Label className="flex flex-col items-stretch gap-1.5">
                    <span>Title</span>
                    <TextInput
                      aria-label="Title"
                      sizing="lg"
                      value={field.state.value}
                      onChange={(changeEvent) => field.handleChange(changeEvent.target.value)}
                    />
                    {titleError === null ? null : (
                      <p className="text-danger text-sm" role="alert">
                        {titleError}
                      </p>
                    )}
                  </Label>
                );
              }}
            </form.AppField>

            <div className="flex flex-wrap gap-3">
              <form.AppField name="startDate">
                {(field) => {
                  const startError =
                    field.state.meta.isTouched || form.state.isSubmitted
                      ? fieldErrorMessage(field.state.meta.errors)
                      : null;

                  return (
                    <Label className="flex flex-1 flex-col items-stretch gap-1.5">
                      <span>Start date</span>
                      <TextInput
                        type="date"
                        aria-label="Start date"
                        value={field.state.value}
                        onChange={(changeEvent) => {
                          const previousDate = field.state.value;
                          field.handleChange(changeEvent.target.value);
                          followStart(
                            changeEvent.target.value,
                            form.getFieldValue('startTime'),
                            previousDate,
                            form.getFieldValue('startTime'),
                          );
                        }}
                      />
                      {startError === null ? null : (
                        <p className="text-danger text-sm" role="alert">
                          {startError}
                        </p>
                      )}
                    </Label>
                  );
                }}
              </form.AppField>
              <form.AppField name="startTime">
                {(field) => (
                  <Label className="flex w-32 flex-col items-stretch gap-1.5">
                    <span>Start time</span>
                    <TextInput
                      type="time"
                      aria-label="Start time"
                      value={field.state.value}
                      onChange={(changeEvent) => {
                        const previousTime = field.state.value;
                        field.handleChange(changeEvent.target.value);
                        followStart(
                          form.getFieldValue('startDate'),
                          changeEvent.target.value,
                          form.getFieldValue('startDate'),
                          previousTime,
                        );
                      }}
                    />
                  </Label>
                )}
              </form.AppField>
            </div>

            <div className="flex flex-wrap gap-3">
              <form.AppField name="endDate">
                {(field) => (
                  <Label className="flex flex-1 flex-col items-stretch gap-1.5">
                    <span>End date</span>
                    <TextInput
                      type="date"
                      aria-label="End date"
                      value={field.state.value}
                      onChange={(changeEvent) => {
                        setEndTouched(true);
                        field.handleChange(changeEvent.target.value);
                      }}
                    />
                  </Label>
                )}
              </form.AppField>
              <form.AppField name="endTime">
                {(field) => (
                  <Label className="flex w-32 flex-col items-stretch gap-1.5">
                    <span>End time</span>
                    <TextInput
                      type="time"
                      aria-label="End time"
                      value={field.state.value}
                      onChange={(changeEvent) => {
                        setEndTouched(true);
                        field.handleChange(changeEvent.target.value);
                      }}
                    />
                  </Label>
                )}
              </form.AppField>
            </div>

            <p className="text-muted text-xs">Leave the start time empty for an all-day event.</p>

            <form.AppField name="description">
              {(field) => (
                <Label className="flex flex-col items-stretch gap-1.5">
                  <span>Description</span>
                  <textarea
                    aria-label="Description"
                    value={field.state.value}
                    onChange={(changeEvent) => field.handleChange(changeEvent.target.value)}
                    className={`${INPUT_CLASS} min-h-24 resize-y`}
                  />
                </Label>
              )}
            </form.AppField>

            {event === null || applyTo === 'all' ? (
              <form.AppField name="recurrence">
                {(field) => (
                  <RecurrenceField
                    value={field.state.value}
                    startDate={startDateValue}
                    onChange={field.handleChange}
                    error={
                      field.state.meta.isTouched || form.state.isSubmitted
                        ? fieldErrorMessage(field.state.meta.errors)
                        : null
                    }
                  />
                )}
              </form.AppField>
            ) : null}

            <div className="flex items-center justify-end">
              <Button type="submit" disabled={isSubmitting || (event !== null && !hasChanges)}>
                {isSubmitting
                  ? 'Saving...'
                  : event === null
                    ? 'Create event'
                    : hasChanges
                      ? 'Save changes'
                      : 'No changes'}
              </Button>
            </div>
          </form>
        </Card>

        {event === null ? null : (
          <div className="border-line mt-auto flex flex-col gap-2 border-t pt-4 sm:flex-row sm:flex-wrap">
            {showingOccurrence ? (
              <>
                <Button color="red" onClick={() => setPendingDelete('occurrence')}>
                  Delete this occurrence
                </Button>
                <Button color="red" onClick={() => setPendingDelete('following')}>
                  Delete this and following
                </Button>
                <Button color="red" onClick={() => setPendingDelete('series')}>
                  Delete series
                </Button>
              </>
            ) : (
              <Button color="red" onClick={() => setPendingDelete('series')}>
                Delete event
              </Button>
            )}
          </div>
        )}
      </div>
      {event !== null && pendingDelete !== null ? (
        <DeleteEventModal
          show
          event={event}
          mode={pendingDelete}
          onClose={() => setPendingDelete(null)}
          onConfirm={
            pendingDelete === 'occurrence'
              ? handleDeleteOccurrence
              : pendingDelete === 'following'
                ? handleDeleteFollowing
                : handleDelete
          }
        />
      ) : null}
    </>
  );
}
