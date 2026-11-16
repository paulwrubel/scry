// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { DateTime } from 'luxon';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DEVICE_ZONE } from '@/contract/due';
import { makeEvent, makeProject } from '@/test/fixtures';
import { DayEvents } from './DayEvents';

const DAY = DateTime.fromISO('2026-11-06', { zone: DEVICE_ZONE });

afterEach(cleanup);

function renderDay(
  project = makeProject(),
  handlers: { onCreate?: () => void; onClose?: () => void } = {},
) {
  const { onCreate = vi.fn(), onClose = vi.fn() } = handlers;

  render(
    <MemoryRouter>
      <DayEvents
        project={project}
        day={DAY}
        monthQuery="?month=2026-11"
        onCreate={onCreate}
        onClose={onClose}
      />
    </MemoryRouter>,
  );

  return { onCreate, onClose };
}

function timed(id: string, start: string, end: string) {
  return makeEvent({
    id,
    title: id,
    interval: {
      start: { kind: 'zoned', datetime: start, zone: DEVICE_ZONE },
      end: { kind: 'zoned', datetime: end, zone: DEVICE_ZONE },
    },
  });
}

describe('DayEvents', () => {
  it('lists the day with all-day events above timed ones', () => {
    renderDay(
      makeProject({
        events: [
          timed('standup', '2026-11-06T09:00:00', '2026-11-06T10:00:00'),
          makeEvent({ id: 'offsite', title: 'offsite' }),
          makeEvent({
            id: 'later',
            title: 'later',
            interval: {
              start: { kind: 'date', date: '2026-11-07' },
              end: { kind: 'date', date: '2026-11-08' },
            },
          }),
        ],
      }),
    );

    expect(screen.getByRole('heading', { name: 'Friday' })).toBeDefined();
    expect(screen.getByText('November 6, 2026')).toBeDefined();

    const items = screen.getAllByRole('listitem');
    expect(items[0].textContent).toContain('offsite');
    expect(items[1].textContent).toContain('standup');
    expect(screen.getByText('All day')).toBeDefined();
    expect(screen.getByText('9:00 AM – 10:00 AM')).toBeDefined();
    expect(screen.queryByText('later')).toBeNull();
  });

  it('says so when the day is empty', () => {
    renderDay(makeProject());

    expect(screen.getByText('No events this day.')).toBeDefined();
  });

  it('opens the editor for a listed event', () => {
    renderDay(
      makeProject({ events: [timed('standup', '2026-11-06T09:00:00', '2026-11-06T10:00:00')] }),
    );

    expect(screen.getByRole('link').getAttribute('href')).toBe(
      '/projects/project-1/events/standup?month=2026-11',
    );
  });

  it('creates an event for the day', () => {
    const { onCreate } = renderDay();

    fireEvent.click(screen.getByRole('button', { name: 'New event' }));

    expect(onCreate).toHaveBeenCalled();
  });

  it('closes back to the calendar', () => {
    const { onClose } = renderDay();

    fireEvent.click(screen.getByRole('button', { name: 'Back to calendar' }));

    expect(onClose).toHaveBeenCalled();
  });
});
