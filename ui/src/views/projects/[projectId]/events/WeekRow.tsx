import type { DateTime } from 'luxon';
import { useElementHeight } from '@/hooks/useElementHeight';
import { dayKey, eventKey, fitLanes } from '@/utils/events/calendar';
import type { Month, WeekLane } from '@/utils/events/calendar';
import { DayCell } from './DayCell';
import { EventBar } from './EventBar';

/// One lane slot, in pixels. The measured height divides into this, and the
/// last whole slot is kept for the overflow count.
const LANE_HEIGHT = 22;

export type WeekRowProps = {
  week: readonly DateTime[];
  lanes: readonly WeekLane[];
  month: Month;
  today: DateTime;
  projectId: string;
  monthQuery: string;
  onShowDay: (day: DateTime) => void;
};

export function WeekRow(props: WeekRowProps) {
  const { week, lanes, month, today, projectId, monthQuery, onShowDay } = props;

  const { ref, height } = useElementHeight<HTMLDivElement>();
  const capacity = height === null || height <= 0 ? null : Math.floor(height / LANE_HEIGHT);
  const { visible, hiddenByDay } = fitLanes(lanes, capacity);

  const todayIndex = week.findIndex((day) => day.hasSame(today, 'day'));

  return (
    <div className="border-line relative flex min-h-0 flex-1 border-b">
      <div className="grid min-h-0 min-w-0 flex-1 grid-cols-7">
        {week.map((day) => (
          <DayCell key={dayKey(day)} day={day} month={month} today={today} onShowDay={onShowDay} />
        ))}
      </div>
      {/* the bars sit above the day buttons but leave their empty space clickable */}
      <div className="pointer-events-none absolute inset-0 pt-7">
        <div ref={ref} className="flex h-full flex-col overflow-hidden">
          {visible.map((lane, laneIndex) => (
            <div key={laneIndex} className="relative" style={{ height: LANE_HEIGHT }}>
              {lane.map((placement) => (
                <EventBar
                  key={eventKey(placement.event)}
                  placement={placement}
                  projectId={projectId}
                  monthQuery={monthQuery}
                />
              ))}
            </div>
          ))}
          {hiddenByDay.some((count) => count > 0) ? (
            <div className="relative" style={{ height: LANE_HEIGHT }}>
              {hiddenByDay.map((count, index) =>
                count === 0 ? null : (
                  <button
                    key={index}
                    type="button"
                    onClick={() => onShowDay(week[index])}
                    aria-label={`Show ${count} more events on ${week[index].toFormat('cccc, LLLL d')}`}
                    style={{ left: `${(index * 100) / 7}%`, width: `${100 / 7}%` }}
                    className="text-muted pointer-events-auto absolute inset-y-0 flex cursor-pointer items-center overflow-hidden px-1.5 text-[11px] leading-none whitespace-nowrap transition-colors hover:text-zinc-200"
                  >
                    +{count} more
                  </button>
                ),
              )}
            </div>
          ) : null}
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
