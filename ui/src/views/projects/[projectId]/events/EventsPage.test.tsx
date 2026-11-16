// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useParams } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DEVICE_ZONE } from '@/contract/due';
import type { Event } from '@/generated/Event';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import { SearchProbe } from '@/test/SearchProbe';
import { createReplicaMock, makeEvent, makeProject, makeReplicaSnapshot } from '@/test/fixtures';
import { EventsPage } from './EventsPage';

const PROJECT_ID = 'project-1';
const MONTH_PATH = `/projects/${PROJECT_ID}/events?month=2026-11`;

afterEach(() => {
  cleanup();
});

function event(id: string, overrides: Partial<Event> = {}): Event {
  return makeEvent({ id, title: id, project_id: PROJECT_ID, ...overrides });
}

function renderEvents(project: ProjectSnapshot | null, path = MONTH_PATH) {
  const context: ReplicaContextValue = {
    status: 'ready',
    replica: createReplicaMock(makeReplicaSnapshot(project === null ? [] : [project]), {
      enqueue: vi.fn(),
    }),
    error: undefined,
    retry: () => undefined,
  };

  render(
    <ReplicaContext.Provider value={context}>
      <MemoryRouter initialEntries={[path]}>
        <SearchProbe />
        <Routes>
          <Route path="/projects/:projectId/events" element={<EventsPage />} />
          <Route path="/projects/:projectId/events/days/:date" element={<DayProbe />} />
        </Routes>
      </MemoryRouter>
    </ReplicaContext.Provider>,
  );
}

describe('EventsPage', () => {
  it('draws a multi-day event as one bar spanning its days', () => {
    renderEvents(
      makeProject({
        events: [
          event('standup', {
            interval: {
              start: { kind: 'zoned', datetime: '2026-11-06T09:00:00', zone: 'UTC' },
              end: { kind: 'zoned', datetime: '2026-11-06T10:00:00', zone: 'UTC' },
            },
          }),
          event('offsite', {
            interval: {
              start: { kind: 'date', date: '2026-11-04' },
              end: { kind: 'date', date: '2026-11-07' },
            },
          }),
        ],
      }),
    );

    expect(screen.getByRole('heading', { name: 'November 2026' })).toBeDefined();

    const standup = screen.getByRole('link', { name: /standup/ });
    expect(standup.textContent).toContain('9:00 AM');
    expect(standup.textContent).toContain('10:00 AM');

    const offsite = screen.getByRole('link', { name: /offsite/ });
    expect(parseFloat(offsite.style.width)).toBeCloseTo((3 * 100) / 7, 3);
    expect(parseFloat(offsite.style.left)).toBeCloseTo((3 * 100) / 7, 3);
  });

  it('splits a bar that crosses a week boundary', () => {
    renderEvents(
      makeProject({
        events: [
          event('offsite', {
            interval: {
              start: { kind: 'date', date: '2026-11-07' },
              end: { kind: 'date', date: '2026-11-10' },
            },
          }),
        ],
      }),
    );

    expect(screen.getAllByRole('link', { name: /offsite/ })).toHaveLength(2);
  });

  it('links an event to its editor', () => {
    renderEvents(makeProject({ events: [event('standup')] }));

    expect(screen.getByRole('link', { name: /standup/ }).getAttribute('href')).toBe(
      `/projects/${PROJECT_ID}/events/standup?month=2026-11`,
    );
  });

  it('opens the day list for a day', () => {
    renderEvents(makeProject());

    fireEvent.click(screen.getByRole('button', { name: 'Show events on Friday, November 6' }));

    expect(screen.getByTestId('day').textContent).toBe('2026-11-06');
    expect(screen.getByTestId('search').textContent).toBe('month=2026-11');
  });

  it('moves to the next month', () => {
    renderEvents(makeProject());

    fireEvent.click(screen.getByRole('button', { name: 'Next month' }));

    expect(screen.getByTestId('search').textContent).toBe('month=2026-12');
    expect(screen.getByRole('heading', { name: 'December 2026' })).toBeDefined();
  });

  it('switches to the week view', () => {
    renderEvents(makeProject(), `/projects/${PROJECT_ID}/events?view=month&month=2026-11`);

    fireEvent.click(screen.getByRole('button', { name: 'Week' }));

    expect(screen.getByTestId('search').textContent).toContain('view=week');
    expect(screen.getByTestId('search').textContent).toContain('week=');
    expect(screen.getByRole('button', { name: 'Week' }).getAttribute('aria-pressed')).toBe('true');
  });

  it('switches back to the month of the visible week', () => {
    renderEvents(
      makeProject(),
      `/projects/${PROJECT_ID}/events?view=week&week=2026-12-06&month=2026-11`,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Month' }));

    expect(screen.getByTestId('search').textContent).toContain('view=month');
    expect(screen.getByTestId('search').textContent).toContain('month=2026-12');
    expect(screen.getByRole('heading', { name: 'December 2026' })).toBeDefined();
  });

  it('moves to the next week', () => {
    renderEvents(makeProject(), `/projects/${PROJECT_ID}/events?view=week&week=2026-11-01`);

    expect(screen.getByRole('heading', { name: 'November 1 – 7, 2026' })).toBeDefined();

    fireEvent.click(screen.getByRole('button', { name: 'Next week' }));

    expect(screen.getByTestId('search').textContent).toBe('view=week&week=2026-11-08');
    expect(screen.getByRole('heading', { name: 'November 8 – 14, 2026' })).toBeDefined();
  });

  it('draws timed and all-day events in the week view', () => {
    renderEvents(
      makeProject({
        events: [
          event('offsite', {
            interval: {
              start: { kind: 'date', date: '2026-11-02' },
              end: { kind: 'date', date: '2026-11-05' },
            },
          }),
          event('standup', {
            interval: {
              start: { kind: 'zoned', datetime: '2026-11-02T09:00:00', zone: DEVICE_ZONE },
              end: { kind: 'zoned', datetime: '2026-11-02T10:00:00', zone: DEVICE_ZONE },
            },
          }),
        ],
      }),
      `/projects/${PROJECT_ID}/events?view=week&week=2026-11-01`,
    );

    expect(screen.getByText('All day')).toBeDefined();
    expect(screen.getByText('12:00 AM')).toBeDefined();
    expect(screen.getAllByRole('link', { name: /offsite/ })).toHaveLength(1);
    expect(screen.getAllByRole('link', { name: /standup/ })).toHaveLength(1);

    const timed = screen.getByRole('link', { name: /standup/ });
    expect(parseFloat(timed.style.top)).toBeCloseTo((540 / 1440) * 100, 3);
  });

  it('opens a day from the week view', () => {
    renderEvents(makeProject(), `/projects/${PROJECT_ID}/events?view=week&week=2026-11-01`);

    fireEvent.click(screen.getByRole('button', { name: 'Show events on Monday, November 2' }));

    expect(screen.getByTestId('day').textContent).toBe('2026-11-02');
    expect(screen.getByTestId('search').textContent).toContain('view=week');
    expect(screen.getByTestId('search').textContent).toContain('week=2026-11-01');
  });

  it('warns when the project is missing', () => {
    renderEvents(null);

    expect(screen.getByText(/Project not found/)).toBeDefined();
  });
});

function DayProbe() {
  const { date } = useParams();
  return <span data-testid="day">{date}</span>;
}
