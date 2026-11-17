import { Button } from 'flowbite-react';
import type { DateTime } from 'luxon';
import { HiPlus } from 'react-icons/hi2';
import { Link } from 'react-router-dom';
import { BackButton } from '@/components/BackButton';
import { DEVICE_ZONE } from '@/contract/due';
import { displayedClock, formatTimeOfDay } from '@/contract/interval';
import type { Event } from '@/generated/Event';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { eventKey, eventPath, eventsOnDay, expandEvents } from '@/utils/events/calendar';

export type DayEventsProps = {
  project: ProjectSnapshot;
  day: DateTime;
  monthQuery: string;
  onCreate: () => void;
  onClose: () => void;
};

/// Every event that covers one day, with the way to add another at the top.
export function DayEvents(props: DayEventsProps) {
  const { project, day, monthQuery, onCreate, onClose } = props;

  const events = eventsOnDay(
    expandEvents(project.events, day, day.plus({ days: 1 }), DEVICE_ZONE),
    day,
    DEVICE_ZONE,
  );

  return (
    <div className="flex min-h-full flex-col gap-4 p-6">
      <div className="flex items-center justify-between gap-2">
        <BackButton label="Back to calendar" onClick={onClose} />
        <Button size="sm" onClick={onCreate}>
          <HiPlus aria-hidden="true" className="mr-1 h-4 w-4" />
          New event
        </Button>
      </div>
      <div className="flex flex-col gap-0.5">
        <h1 className="text-2xl font-semibold">{day.toFormat('cccc')}</h1>
        <p className="text-muted text-sm">{day.toFormat('LLLL d, yyyy')}</p>
      </div>
      {events.length === 0 ? (
        <p className="text-faint text-sm">No events this day.</p>
      ) : (
        <ul className="flex flex-col gap-2">
          {events.map((event) => (
            <li key={eventKey(event)}>
              <Link
                to={eventPath(project.id, event, monthQuery)}
                state={{ animateWidth: true }}
                className="border-line bg-panel hover:border-line-hover flex items-center gap-3 rounded-lg border px-3 py-2 transition-colors"
              >
                <span className="text-muted w-32 shrink-0 text-xs">{timeLabel(event)}</span>
                <span className="min-w-0 flex-1 truncate text-sm text-zinc-200">{event.title}</span>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function timeLabel(event: Event): string {
  const start = displayedClock(event.interval.start, DEVICE_ZONE);
  if (start === null) {
    return 'All day';
  }

  const end = displayedClock(event.interval.end, DEVICE_ZONE);
  return end === null
    ? formatTimeOfDay(start)
    : `${formatTimeOfDay(start)} – ${formatTimeOfDay(end)}`;
}
