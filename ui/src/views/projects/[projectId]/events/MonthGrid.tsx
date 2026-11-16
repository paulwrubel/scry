import type { DateTime } from 'luxon';
import { dayKey } from '@/utils/events/calendar';
import type { Month, WeekLane } from '@/utils/events/calendar';
import { WeekRow } from './WeekRow';

const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'] as const;

export type MonthGridProps = {
  weeks: readonly DateTime[][];
  lanes: readonly WeekLane[][];
  month: Month;
  today: DateTime;
  projectId: string;
  monthQuery: string;
  onSelectDay: (day: DateTime) => void;
};

export function MonthGrid(props: MonthGridProps) {
  const { weeks, lanes, month, today, projectId, monthQuery, onSelectDay } = props;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="border-line text-muted grid grid-cols-7 border-b text-center text-xs font-medium">
        {WEEKDAYS.map((weekday) => (
          <div key={weekday} className="py-1">
            {weekday}
          </div>
        ))}
      </div>
      <div className="flex min-h-0 flex-1 flex-col">
        {weeks.map((week, index) => (
          <WeekRow
            key={dayKey(week[0])}
            week={week}
            lanes={lanes[index]}
            month={month}
            today={today}
            projectId={projectId}
            monthQuery={monthQuery}
            onSelectDay={onSelectDay}
          />
        ))}
      </div>
    </div>
  );
}
