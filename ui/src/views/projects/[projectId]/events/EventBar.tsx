import { Link } from 'react-router-dom';
import { formatTimeOfDay } from '@/contract/interval';
import type { EventPlacement } from '@/utils/events/calendar';

export type EventBarProps = {
  placement: EventPlacement;
  projectId: string;
  monthQuery: string;
};

export function EventBar(props: EventBarProps) {
  const { placement, projectId, monthQuery } = props;

  const { event, startIndex, endIndex, continuesBefore, continuesAfter, startTime, endTime } =
    placement;
  const days = endIndex - startIndex + 1;

  return (
    <Link
      to={`/projects/${projectId}/events/${event.id}${monthQuery}`}
      title={event.title}
      style={{ left: `${(startIndex * 100) / 7}%`, width: `${(days * 100) / 7}%` }}
      className={`bg-panel hover:bg-line pointer-events-auto absolute inset-y-px flex items-center gap-1 overflow-hidden px-1.5 text-[11px] leading-none transition-colors ${
        continuesBefore ? '' : 'rounded-l'
      } ${continuesAfter ? '' : 'rounded-r'}`}
    >
      {startTime === null ? null : (
        <span className="text-muted shrink-0">{formatTimeOfDay(startTime)}</span>
      )}
      <span className="min-w-0 flex-1 truncate">{event.title}</span>
      {endTime === null ? null : (
        <span className="text-muted shrink-0">{formatTimeOfDay(endTime)}</span>
      )}
    </Link>
  );
}
