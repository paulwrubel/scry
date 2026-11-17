import {
  Button,
  Checkbox,
  Label,
  Modal,
  ModalBody,
  ModalFooter,
  ModalHeader,
  Select,
  TextInput,
} from 'flowbite-react';
import { DateTime } from 'luxon';
import { fieldErrorMessage } from '@/components/form-controls/fieldErrors';
import { useAppForm } from '@/hooks/useAppForm';
import { useCloseRequest } from '@/hooks/useCloseRequest';
import type { RecurrenceDraft } from '@/utils/events/recurrence';
import { rruleFromDraft } from '@/utils/events/recurrence';
import {
  FREQUENCIES,
  MONTHS,
  ORDINALS,
  UNITS,
  WEEKDAYS,
  parseEnd,
  parseFrequency,
  parseMonthlyMode,
  parseYearlyMode,
  recurrenceSchema,
} from './RecurrenceDialog.schema';

export type RecurrenceDialogProps = {
  show: boolean;
  startDate: string;
  initial: RecurrenceDraft;
  canRemove: boolean;
  onApply: (value: string) => void;
  onRemove: () => void;
  onClose: () => void;
};

/// The classic Outlook recurrence dialog. It edits a draft and writes the
/// generated RRULE back to the parent form; the text itself is never editable.
export function RecurrenceDialog(props: RecurrenceDialogProps) {
  const { show, startDate, initial, canRemove, onApply, onRemove, onClose } = props;

  useCloseRequest(show, onClose);

  const startWeekday = DateTime.fromISO(startDate).weekday || 1;
  const form = useAppForm({
    defaultValues: initial,
    validators: { onChange: recurrenceSchema, onMount: recurrenceSchema },
    onSubmit: ({ value }) => {
      onApply(rruleFromDraft(value));
    },
  });

  return (
    <Modal show={show} onClose={onClose} dismissible>
      <form
        id="recurrence-form"
        onSubmit={(event) => {
          event.preventDefault();
          void form.handleSubmit();
        }}
      >
        <ModalHeader>Recurrence</ModalHeader>
        <ModalBody className="flex max-h-[70vh] flex-col gap-5 overflow-y-auto">
          <div className="flex flex-wrap items-end gap-3">
            <form.AppField name="frequency">
              {(field) => (
                <Label className="flex flex-1 flex-col items-stretch gap-1.5">
                  <span>Occurs</span>
                  <Select
                    value={field.state.value}
                    onChange={(event) => field.handleChange(parseFrequency(event.target.value))}
                  >
                    {FREQUENCIES.map((frequency) => (
                      <option key={frequency.value} value={frequency.value}>
                        {frequency.label}
                      </option>
                    ))}
                  </Select>
                </Label>
              )}
            </form.AppField>
            <form.AppField name="interval">
              {(field) => (
                <Label className="flex w-20 flex-col items-stretch gap-1.5">
                  <span>Every</span>
                  <TextInput
                    type="number"
                    min={1}
                    max={99}
                    value={field.state.value}
                    onChange={(event) => field.handleChange(Number(event.target.value))}
                  />
                </Label>
              )}
            </form.AppField>
            <form.Subscribe selector={(state) => state.values.frequency}>
              {(frequency) => <span className="pb-2 text-sm">{UNITS[frequency]}</span>}
            </form.Subscribe>
          </div>

          <form.Subscribe selector={(state) => state.values.frequency}>
            {(frequency) =>
              frequency === 'weekly' ? (
                <div className="flex flex-col gap-3">
                  <form.AppField name="weeklyDays">
                    {(field) => (
                      <div className="flex flex-col gap-1.5">
                        <span className="text-sm">On</span>
                        <div className="flex flex-wrap gap-3">
                          {WEEKDAYS.map((weekday) => (
                            <Label key={weekday.value} className="flex items-center gap-1.5">
                              <Checkbox
                                checked={field.state.value.includes(weekday.value)}
                                disabled={weekday.value === startWeekday}
                                onChange={(event) => {
                                  const next = event.target.checked
                                    ? [...field.state.value, weekday.value]
                                    : field.state.value.filter(
                                        (candidate) => candidate !== weekday.value,
                                      );
                                  field.handleChange(next);
                                }}
                              />
                              <span>{weekday.label}</span>
                            </Label>
                          ))}
                        </div>
                        {field.state.meta.errors.length === 0 ? null : (
                          <p className="text-danger text-sm" role="alert">
                            {fieldErrorMessage(field.state.meta.errors)}
                          </p>
                        )}
                      </div>
                    )}
                  </form.AppField>
                  <form.AppField name="weeklyStart">
                    {(field) => (
                      <Label className="flex max-w-xs flex-col items-stretch gap-1.5">
                        <span>Week starts on</span>
                        <Select
                          value={field.state.value}
                          onChange={(event) => field.handleChange(Number(event.target.value))}
                        >
                          {WEEKDAYS.map((weekday) => (
                            <option key={weekday.value} value={weekday.value}>
                              {weekday.label}
                            </option>
                          ))}
                        </Select>
                      </Label>
                    )}
                  </form.AppField>
                </div>
              ) : frequency === 'monthly' ? (
                <div className="flex flex-col gap-3">
                  <form.AppField name="monthlyMode">
                    {(field) => (
                      <Label className="flex flex-col items-stretch gap-1.5">
                        <span>Pattern</span>
                        <Select
                          value={field.state.value}
                          onChange={(event) =>
                            field.handleChange(parseMonthlyMode(event.target.value))
                          }
                        >
                          <option value="day">Day of the month</option>
                          <option value="weekday">Weekday of the month</option>
                        </Select>
                      </Label>
                    )}
                  </form.AppField>
                  <form.Subscribe selector={(state) => state.values.monthlyMode}>
                    {(mode) =>
                      mode === 'day' ? (
                        <form.AppField name="monthlyDay">
                          {(field) => (
                            <Label className="flex max-w-xs flex-col items-stretch gap-1.5">
                              <span>Day</span>
                              <TextInput
                                type="number"
                                min={1}
                                max={31}
                                value={field.state.value}
                                onChange={(event) => field.handleChange(Number(event.target.value))}
                              />
                            </Label>
                          )}
                        </form.AppField>
                      ) : (
                        <div className="flex flex-wrap items-end gap-3">
                          <form.AppField name="monthlyOrdinal">
                            {(field) => (
                              <Label className="flex flex-col items-stretch gap-1.5">
                                <span>Which</span>
                                <Select
                                  value={field.state.value}
                                  onChange={(event) =>
                                    field.handleChange(Number(event.target.value))
                                  }
                                >
                                  {ORDINALS.map((ordinal) => (
                                    <option key={ordinal.value} value={ordinal.value}>
                                      {ordinal.label}
                                    </option>
                                  ))}
                                </Select>
                              </Label>
                            )}
                          </form.AppField>
                          <form.AppField name="monthlyWeekday">
                            {(field) => (
                              <Label className="flex flex-col items-stretch gap-1.5">
                                <span>Weekday</span>
                                <Select
                                  value={field.state.value}
                                  onChange={(event) =>
                                    field.handleChange(Number(event.target.value))
                                  }
                                >
                                  {WEEKDAYS.map((weekday) => (
                                    <option key={weekday.value} value={weekday.value}>
                                      {weekday.label}
                                    </option>
                                  ))}
                                </Select>
                              </Label>
                            )}
                          </form.AppField>
                        </div>
                      )
                    }
                  </form.Subscribe>
                </div>
              ) : frequency === 'yearly' ? (
                <div className="flex flex-col gap-3">
                  <form.AppField name="yearlyMode">
                    {(field) => (
                      <Label className="flex flex-col items-stretch gap-1.5">
                        <span>Pattern</span>
                        <Select
                          value={field.state.value}
                          onChange={(event) =>
                            field.handleChange(parseYearlyMode(event.target.value))
                          }
                        >
                          <option value="date">Month and day</option>
                          <option value="weekday">Weekday of the month</option>
                        </Select>
                      </Label>
                    )}
                  </form.AppField>
                  <div className="flex flex-wrap items-end gap-3">
                    <form.AppField name="yearlyMonth">
                      {(field) => (
                        <Label className="flex flex-col items-stretch gap-1.5">
                          <span>Month</span>
                          <Select
                            value={field.state.value}
                            onChange={(event) => field.handleChange(Number(event.target.value))}
                          >
                            {MONTHS.map((month) => (
                              <option key={month.value} value={month.value}>
                                {month.label}
                              </option>
                            ))}
                          </Select>
                        </Label>
                      )}
                    </form.AppField>
                    <form.Subscribe selector={(state) => state.values.yearlyMode}>
                      {(mode) =>
                        mode === 'date' ? (
                          <form.AppField name="yearlyDay">
                            {(field) => (
                              <Label className="flex w-20 flex-col items-stretch gap-1.5">
                                <span>Day</span>
                                <TextInput
                                  type="number"
                                  min={1}
                                  max={31}
                                  value={field.state.value}
                                  onChange={(event) =>
                                    field.handleChange(Number(event.target.value))
                                  }
                                />
                              </Label>
                            )}
                          </form.AppField>
                        ) : (
                          <>
                            <form.AppField name="yearlyOrdinal">
                              {(field) => (
                                <Label className="flex flex-col items-stretch gap-1.5">
                                  <span>Which</span>
                                  <Select
                                    value={field.state.value}
                                    onChange={(event) =>
                                      field.handleChange(Number(event.target.value))
                                    }
                                  >
                                    {ORDINALS.map((ordinal) => (
                                      <option key={ordinal.value} value={ordinal.value}>
                                        {ordinal.label}
                                      </option>
                                    ))}
                                  </Select>
                                </Label>
                              )}
                            </form.AppField>
                            <form.AppField name="yearlyWeekday">
                              {(field) => (
                                <Label className="flex flex-col items-stretch gap-1.5">
                                  <span>Weekday</span>
                                  <Select
                                    value={field.state.value}
                                    onChange={(event) =>
                                      field.handleChange(Number(event.target.value))
                                    }
                                  >
                                    {WEEKDAYS.map((weekday) => (
                                      <option key={weekday.value} value={weekday.value}>
                                        {weekday.label}
                                      </option>
                                    ))}
                                  </Select>
                                </Label>
                              )}
                            </form.AppField>
                          </>
                        )
                      }
                    </form.Subscribe>
                  </div>
                </div>
              ) : null
            }
          </form.Subscribe>

          <div className="flex flex-col gap-3">
            <form.AppField name="endMode">
              {(field) => (
                <Label className="flex max-w-xs flex-col items-stretch gap-1.5">
                  <span>Range</span>
                  <Select
                    value={field.state.value}
                    onChange={(event) => field.handleChange(parseEnd(event.target.value))}
                  >
                    <option value="never">No end</option>
                    <option value="count">End after occurrences</option>
                    <option value="until">End by date</option>
                  </Select>
                </Label>
              )}
            </form.AppField>
            <form.Subscribe selector={(state) => state.values.endMode}>
              {(mode) =>
                mode === 'count' ? (
                  <form.AppField name="count">
                    {(field) => (
                      <Label className="flex max-w-xs flex-col items-stretch gap-1.5">
                        <span>Occurrences</span>
                        <TextInput
                          type="number"
                          min={1}
                          value={field.state.value}
                          onChange={(event) => field.handleChange(Number(event.target.value))}
                        />
                      </Label>
                    )}
                  </form.AppField>
                ) : mode === 'until' ? (
                  <form.AppField name="until">
                    {(field) => (
                      <Label className="flex max-w-xs flex-col items-stretch gap-1.5">
                        <span>End date</span>
                        <TextInput
                          type="date"
                          value={field.state.value}
                          onChange={(event) => field.handleChange(event.target.value)}
                        />
                      </Label>
                    )}
                  </form.AppField>
                ) : null
              }
            </form.Subscribe>
          </div>

          <form.Subscribe selector={(state) => state.values}>
            {(values) => (
              <div className="flex flex-col gap-1">
                <span className="text-muted text-xs">RRULE</span>
                <code className="text-faint overflow-x-auto text-xs">{rruleFromDraft(values)}</code>
              </div>
            )}
          </form.Subscribe>
        </ModalBody>
        <ModalFooter className="justify-between">
          <Button
            color="alternative"
            type="button"
            disabled={!canRemove}
            onClick={() => {
              onRemove();
              onClose();
            }}
          >
            Remove recurrence
          </Button>
          <div className="flex items-center gap-2">
            <Button color="alternative" type="button" onClick={onClose}>
              Cancel
            </Button>
            <form.Subscribe selector={(state) => state.isValid}>
              {(isValid) => (
                <Button type="submit" disabled={!isValid}>
                  OK
                </Button>
              )}
            </form.Subscribe>
          </div>
        </ModalFooter>
      </form>
    </Modal>
  );
}
