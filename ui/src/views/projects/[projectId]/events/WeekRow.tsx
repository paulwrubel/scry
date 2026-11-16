import type { DateTime } from 'luxon';
import { dayKey } from '@/utils/events/calendar';
import type { Month, WeekLane } from '@/utils/events/calendar';
import { DayCell } from './DayCell';
import { EventBar } from './EventBar';

const LANE_HEIGHT = 'h-5';

export type WeekRowProps = {
  week: readonly DateTime[];
  lanes: readonly WeekLane[];
  month: Month;
  today: DateTime;
  projectId: string;
  monthQuery: string;
  onSelectDay: (day: DateTime) => void;
};

export function WeekRow(props: WeekRowProps) {
  const { week, lanes, month, today, projectId, monthQuery, onSelectDay } = props;

  const todayIndex = week.findIndex((day) => day.hasSame(today, 'day'));

  return (
    <div className="border-line relative flex min-h-0 flex-1 border-b">
      <div className="grid min-h-0 min-w-0 flex-1 grid-cols-7">
        {week.map((day) => (
          <DayCell
            key={dayKey(day)}
            day={day}
            month={month}
            today={today}
            onSelectDay={onSelectDay}
          />
        ))}
      </div>
      {/* the bars sit above the day buttons but leave their empty space clickable */}
      <div className="pointer-events-none absolute inset-0 pt-7">
        <div className="flex flex-col gap-0.5 pt-0.5">
          {lanes.map((lane, laneIndex) => (
            <div key={laneIndex} className={`relative ${LANE_HEIGHT}`}>
              {lane.map((placement) => (
                <EventBar
                  key={placement.event.id}
                  placement={placement}
                  projectId={projectId}
                  monthQuery={monthQuery}
                />
              ))}
            </div>
          ))}
        </div>
      </div>
      {/* today's outline comes last so a spanned bar cannot paint over it */}
      {todayIndex === -1 ? null : (
        <div aria-hidden="true" className="pointer-events-none absolute inset-0 grid grid-cols-7">
          {week.map((day, index) => (
            <div
              key={dayKey(day)}
              className={index === todayIndex ? 'border-2 border-blue-500' : undefined}
            />
          ))}
        </div>
      )}
    </div>
  );
}
