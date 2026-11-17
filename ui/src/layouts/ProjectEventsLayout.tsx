import { Drawer } from 'flowbite-react';
import { DateTime } from 'luxon';
import { useState } from 'react';
import { useLocation, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { DEVICE_ZONE } from '@/contract/due';
import type { Event } from '@/generated/Event';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { useReplicaSnapshot } from '@/providers/Replica';
import { dayKey, parseOccurrence } from '@/utils/events/calendar';
import { DayEvents } from '@/views/projects/[projectId]/events/DayEvents';
import { EventsPage } from '@/views/projects/[projectId]/events/EventsPage';
import { EventDetail } from '@/views/projects/[projectId]/events/[eventId]/EventDetail';
import { dayDrawerTheme, drawerTheme } from './drawerTheme';

type Panel =
  | { kind: 'event'; project: ProjectSnapshot; event: Event | null; defaultDate: string | null }
  | { kind: 'day'; project: ProjectSnapshot; day: DateTime };

export function ProjectEventsLayout() {
  const { projectId, eventId, date } = useParams();
  const { visible } = useReplicaSnapshot();
  const navigate = useNavigate();
  const { state } = useLocation();
  const [searchParams] = useSearchParams();
  const [lastPanel, setLastPanel] = useState<Panel | null>(null);

  const project = visible.projects.find((candidate) => candidate.id === projectId);
  const creating = eventId === 'new';
  const event = creating
    ? undefined
    : project?.events.find((candidate) => candidate.id === eventId);
  const day = date === undefined ? undefined : parseDay(date);
  const occurrence = parseOccurrence(searchParams.get('occurrence'));
  const occurrenceKey = occurrence === null ? '' : JSON.stringify(occurrence);
  const open = project !== undefined && (creating || event !== undefined || day !== undefined);

  const live: Panel | null =
    project === undefined || !open
      ? null
      : day !== undefined
        ? { kind: 'day', project, day }
        : {
            kind: 'event',
            project,
            event: event ?? null,
            defaultDate: searchParams.get('date'),
          };
  const panel = live ?? lastPanel;

  // the visible month rides along so closing the drawer returns to it
  function calendarPath(target: string, extra: string): string {
    const params = new URLSearchParams(searchParams);
    params.delete('date');
    params.delete('occurrence');
    const query = [params.toString(), extra].filter((part) => part.length > 0).join('&');
    return `${target}${query.length === 0 ? '' : `?${query}`}`;
  }

  function close() {
    if (live !== null) {
      setLastPanel(live);
    }
    navigate(calendarPath(`/projects/${projectId}/events`, ''));
  }

  function createOn(target: DateTime) {
    navigate(calendarPath(`/projects/${projectId}/events/new`, `date=${dayKey(target)}`));
  }

  return (
    <div className="relative flex min-h-0 min-w-0 flex-1">
      <div className="flex min-w-0 flex-1" inert={open}>
        <EventsPage />
      </div>
      <div
        aria-hidden="true"
        onClick={close}
        className={`fixed inset-0 z-30 bg-black/40 transition-opacity duration-300 ${
          open ? 'opacity-100' : 'pointer-events-none opacity-0'
        }`}
      />
      <Drawer
        open={open}
        position="bottom"
        backdrop={false}
        theme={panel?.kind === 'day' ? dayDrawerTheme : drawerTheme}
        className={
          open && wantsWidthTransition(state)
            ? 'transition-[width,transform] duration-300 ease-out'
            : 'transition-transform duration-300 ease-out'
        }
        onClose={close}
        aria-hidden={!open}
        inert={!open}
      >
        {panel === null ? null : panel.kind === 'day' ? (
          <DayEvents
            key={`day:${dayKey(panel.day)}:${open}`}
            project={panel.project}
            day={panel.day}
            monthQuery={monthQuery(searchParams)}
            onCreate={() => createOn(panel.day)}
            onClose={close}
          />
        ) : (
          <EventDetail
            key={`${panel.event?.id ?? 'new'}:${occurrenceKey}:${open}`}
            project={panel.project}
            event={panel.event}
            defaultDate={panel.defaultDate}
            occurrence={occurrence}
            onSaved={(id) => navigate(calendarPath(`/projects/${projectId}/events/${id}`, ''))}
            onClose={close}
          />
        )}
      </Drawer>
    </div>
  );
}

function parseDay(value: string): DateTime | undefined {
  const day = DateTime.fromISO(value, { zone: DEVICE_ZONE });
  return day.isValid ? day.startOf('day') : undefined;
}

function monthQuery(search: URLSearchParams): string {
  const params = new URLSearchParams(search);
  params.delete('date');
  params.delete('occurrence');
  const query = params.toString();
  return query.length === 0 ? '' : `?${query}`;
}

/// The day list sets this on the links to an event, because moving from one
/// panel to the other changes the drawer's width while it is open. Any other
/// navigation leaves the width change instant, so opening a panel never
/// animates from whichever panel came before it.
function wantsWidthTransition(state: unknown): boolean {
  return (
    typeof state === 'object' &&
    state !== null &&
    'animateWidth' in state &&
    state.animateWidth === true
  );
}
