import type { DateTime } from 'luxon';
import type { Month } from '@/utils/events/calendar';

export type DayCellProps = {
  day: DateTime;
  month: Month;
  today: DateTime;
  onShowDay: (day: DateTime) => void;
};

/// The whole day is the create target, not just the number, and today is marked
/// with a border around the cell. Events are drawn over this by `WeekRow`.
export function DayCell(props: DayCellProps) {
  const { day, month, today, onShowDay } = props;

  const inMonth = day.month === month.month && day.year === month.year;
  const isToday = day.hasSame(today, 'day');
  const label = day.toFormat('cccc, LLLL d');

  return (
    <button
      type="button"
      aria-label={`Show events on ${label}`}
      aria-current={isToday ? 'date' : undefined}
      title={`Show events on ${label}`}
      onClick={() => onShowDay(day)}
      className={`border-line hover:bg-panel/60 flex min-w-0 cursor-pointer flex-col items-start border-r p-1 text-left transition-colors ${
        inMonth ? '' : 'opacity-50'
      }`}
    >
      <span
        className={`inline-flex h-6 min-w-6 items-center justify-center rounded px-1 text-xs font-medium ${
          isToday ? 'font-semibold text-white' : 'text-zinc-300'
        }`}
      >
        {day.day}
      </span>
    </button>
  );
}
