import { Drawer } from 'flowbite-react';
import { useState } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router-dom';
import type { Event } from '@/generated/Event';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { useReplicaSnapshot } from '@/providers/Replica';
import { EventsPage } from '@/views/projects/[projectId]/events/EventsPage';
import { EventDetail } from '@/views/projects/[projectId]/events/[eventId]/EventDetail';
import { drawerTheme } from './drawerTheme';

type Panel = { project: ProjectSnapshot; event: Event | null; defaultDate: string | null };

export function ProjectEventsLayout() {
  const { projectId, eventId } = useParams();
  const { visible } = useReplicaSnapshot();
  const navigate = useNavigate();
  const { search } = useLocation();
  const [lastPanel, setLastPanel] = useState<Panel | null>(null);

  const project = visible.projects.find((candidate) => candidate.id === projectId);
  const creating = eventId === 'new';
  const event = creating
    ? undefined
    : project?.events.find((candidate) => candidate.id === eventId);
  const open = project !== undefined && (creating || event !== undefined);
  const live: Panel | null =
    project === undefined || !open
      ? null
      : {
          project,
          event: event ?? null,
          defaultDate: new URLSearchParams(search).get('date'),
        };
  const panel = live ?? lastPanel;

  // the visible month rides along so closing the drawer returns to it
  function leave(target: string) {
    const params = new URLSearchParams(search);
    params.delete('date');
    const query = params.toString();
    navigate(`${target}${query.length === 0 ? '' : `?${query}`}`);
  }

  function close() {
    if (live !== null) {
      setLastPanel(live);
    }
    leave(`/projects/${projectId}/events`);
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
        theme={drawerTheme}
        onClose={close}
        aria-hidden={!open}
        inert={!open}
      >
        {panel === null ? null : (
          <EventDetail
            key={`${panel.event?.id ?? 'new'}:${open}`}
            project={panel.project}
            event={panel.event}
            defaultDate={panel.defaultDate}
            onSaved={(id) => leave(`/projects/${projectId}/events/${id}`)}
            onClose={close}
          />
        )}
      </Drawer>
    </div>
  );
}
