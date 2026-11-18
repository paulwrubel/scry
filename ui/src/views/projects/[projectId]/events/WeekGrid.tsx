import { DateTime } from 'luxon';
import { useEffect, useRef } from 'react';
import { Link } from 'react-router-dom';
import { DEVICE_ZONE } from '@/contract/due';
import { formatTimeOfDay } from '@/contract/interval';
import type { Event } from '@/generated/Event';
import {
  dayKey,
  eventKey,
  eventPath,
  expandEvents,
  placeTimedWeek,
  placeWeek,
  spanningEvents,
} from '@/utils/events/calendar';
import { EventBar } from './EventBar';

const HOUR_HEIGHT = 96;
const WORK_HOUR_START = 8;
const WORK_HOUR_END = 17;
const HOURS = Array.from({ length: 24 }, (_, hour) => hour);

export type WeekGridProps = {
  week: readonly DateTime[];
  events: readonly Event[];
  today: DateTime;
  projectId: string;
  query: string;
  onShowDay: (day: DateTime) => void;
};

export function WeekGrid(props: WeekGridProps) {
  const { week, events, today, projectId, query, onShowDay } = props;

  const expanded = expandEvents(events, week[0], week[6].plus({ days: 1 }), DEVICE_ZONE);

  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current !== null) {
      scrollRef.current.scrollTop = 8 * HOUR_HEIGHT;
    }
  }, []);

  const querySuffix = query.length === 0 ? '' : `?${query}`;
  const banners = placeWeek(spanningEvents(expanded, DEVICE_ZONE), week, DEVICE_ZONE);
  const timedByDay = placeTimedWeek(expanded, week, DEVICE_ZONE);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="border-line grid grid-cols-[3.5rem_repeat(7,minmax(0,1fr))] border-b">
        <div />
        {week.map((day) => {
          const isToday = day.hasSame(today, 'day');

          return (
            <button
              key={dayKey(day)}
              type="button"
              aria-label={`Show events on ${day.toFormat('cccc, LLLL d')}`}
              aria-current={isToday ? 'date' : undefined}
              onClick={() => onShowDay(day)}
              className="border-line hover:bg-panel/60 flex flex-col items-center gap-0.5 border-l px-1 py-1 text-xs transition-colors"
            >
              <span className="text-muted">{day.toFormat('ccc')}</span>
              <span
                className={`inline-flex h-6 min-w-6 items-center justify-center rounded px-1 font-medium ${
                  isToday ? 'bg-blue-500 font-semibold text-white' : 'text-zinc-300'
                }`}
              >
                {day.day}
              </span>
            </button>
          );
        })}
      </div>
      <div className="border-line grid grid-cols-[3.5rem_repeat(7,minmax(0,1fr))] border-b">
        <div className="text-muted px-2 py-1 text-right text-[11px]">All day</div>
        <div className="col-span-7 max-h-28 overflow-y-auto">
          {banners.length === 0 ? (
            <div className="h-6" />
          ) : (
            banners.map((lane, laneIndex) => (
              <div key={laneIndex} className="relative" style={{ height: 22 }}>
                {lane.map((placement) => (
                  <EventBar
                    key={eventKey(placement.event)}
                    placement={placement}
                    projectId={projectId}
                    monthQuery={querySuffix}
                  />
                ))}
              </div>
            ))
          )}
        </div>
      </div>
      <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto">
        <div
          className="relative grid grid-cols-[3.5rem_repeat(7,minmax(0,1fr))]"
          style={{ height: 24 * HOUR_HEIGHT }}
        >
          <div className="relative">
            {HOURS.map((hour) => (
              <div
                key={hour}
                className={`text-muted absolute right-2 text-[11px] leading-none ${
                  hour === 0 ? '' : '-translate-y-1/2'
                }`}
                style={{ top: hour * HOUR_HEIGHT }}
              >
                {formatTimeOfDay(DateTime.fromObject({ hour }))}
              </div>
            ))}
          </div>
          {week.map((day, index) => {
            const isToday = day.hasSame(today, 'day');
            const isWeekday = day.weekday <= 5;

            return (
              <div
                key={dayKey(day)}
                className={`border-line relative min-w-0 border-l ${isToday ? 'bg-panel/20' : ''}`}
              >
                {isWeekday && (
                  <div
                    aria-hidden="true"
                    data-testid="work-hours-band"
                    className="pointer-events-none absolute inset-x-0 bg-white/3"
                    style={{
                      top: WORK_HOUR_START * HOUR_HEIGHT,
                      height: (WORK_HOUR_END - WORK_HOUR_START) * HOUR_HEIGHT,
                    }}
                  />
                )}
                {HOURS.map((hour) => (
                  <div
                    key={hour}
                    className="border-line/80 absolute inset-x-0 border-t"
                    style={{ top: hour * HOUR_HEIGHT }}
                  />
                ))}
                {HOURS.map((hour) => (
                  <div
                    key={hour}
                    aria-hidden="true"
                    data-testid="half-hour-line"
                    className="border-line-strong absolute inset-x-0 border-t border-dashed"
                    style={{ top: hour * HOUR_HEIGHT + HOUR_HEIGHT / 2 }}
                  />
                ))}
                {timedByDay[index].map((segment) => (
                  <Link
                    key={eventKey(segment.event)}
                    to={eventPath(projectId, segment.event, querySuffix)}
                    title={segment.event.title}
                    style={{
                      top: `${(segment.startMinute / 1440) * 100}%`,
                      height: `${((segment.endMinute - segment.startMinute) / 1440) * 100}%`,
                      left: `${(segment.column / segment.columns) * 100}%`,
                      width: `${100 / segment.columns}%`,
                    }}
                    className="bg-panel hover:bg-line absolute z-10 min-h-4 overflow-hidden rounded border-l-2 border-blue-400 px-1 text-[11px] leading-tight transition-colors"
                  >
                    <span className="block truncate font-medium">{segment.event.title}</span>
                    <span className="text-muted block truncate">
                      {clockLabel(segment.startMinute)}
                    </span>
                  </Link>
                ))}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function clockLabel(startMinute: number): string {
  return formatTimeOfDay(
    DateTime.fromObject({ hour: Math.floor(startMinute / 60), minute: startMinute % 60 }),
  );
}
