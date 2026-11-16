// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { DateTime } from 'luxon';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import { DEVICE_ZONE } from '@/contract/due';
import type { Event } from '@/generated/Event';
import type { Interval } from '@/generated/Interval';
import { makeEvent } from '@/test/fixtures';
import { WeekGrid } from './WeekGrid';

const ZONE = DEVICE_ZONE;
const WEEK = Array.from({ length: 7 }, (_, index) =>
  DateTime.fromISO('2026-11-01', { zone: ZONE }).plus({ days: index }),
);
const TODAY = DateTime.fromISO('2026-11-20', { zone: ZONE });

afterEach(cleanup);

function dateInterval(start: string, end: string): Interval {
  return {
    start: { kind: 'date', date: start },
    end: { kind: 'date', date: end },
  };
}

function timedInterval(start: string, end: string, zone: string = ZONE): Interval {
  return {
    start: { kind: 'zoned', datetime: start, zone },
    end: { kind: 'zoned', datetime: end, zone },
  };
}

function renderGrid(
  events: readonly Event[],
  onShowDay: (day: DateTime) => void = () => undefined,
) {
  render(
    <MemoryRouter>
      <WeekGrid
        week={WEEK}
        events={events}
        today={TODAY}
        projectId="project-1"
        query="view=week&week=2026-11-01"
        onShowDay={onShowDay}
      />
    </MemoryRouter>,
  );
}

describe('WeekGrid', () => {
  it('splits spanning events from timed ones', () => {
    renderGrid([
      makeEvent({
        id: 'offsite',
        title: 'Offsite',
        interval: dateInterval('2026-11-02', '2026-11-05'),
      }),
      makeEvent({
        id: 'standup',
        title: 'Standup',
        interval: timedInterval('2026-11-02T09:00:00', '2026-11-02T10:00:00'),
      }),
    ]);

    const spanningLinks = screen.getAllByRole('link', { name: /Offsite/ });
    expect(spanningLinks).toHaveLength(1);
    const spanningLink = spanningLinks[0];
    expect(spanningLink.style.top).toBe('');
    expect(parseFloat(spanningLink.style.left)).toBeCloseTo((1 * 100) / 7, 3);
    expect(parseFloat(spanningLink.style.width)).toBeCloseTo((3 * 100) / 7, 3);

    const timedLinks = screen.getAllByRole('link', { name: /Standup/ });
    expect(timedLinks).toHaveLength(1);
    const timedLink = timedLinks[0];
    expect(parseFloat(timedLink.style.top)).toBeCloseTo((540 / 1440) * 100, 3);
    expect(parseFloat(timedLink.style.height)).toBeCloseTo((60 / 1440) * 100, 3);
  });

  it('lays overlapping timed events side by side', () => {
    renderGrid([
      makeEvent({
        id: 'standup',
        title: 'Standup',
        interval: timedInterval('2026-11-02T09:00:00', '2026-11-02T10:00:00'),
      }),
      makeEvent({
        id: 'review',
        title: 'Review',
        interval: timedInterval('2026-11-02T09:30:00', '2026-11-02T10:30:00'),
      }),
    ]);

    const standup = screen.getByRole('link', { name: /Standup/ });
    const review = screen.getByRole('link', { name: /Review/ });

    expect(parseFloat(standup.style.width)).toBeCloseTo(50, 3);
    expect(parseFloat(standup.style.left)).toBeCloseTo(0, 3);
    expect(parseFloat(review.style.width)).toBeCloseTo(50, 3);
    expect(parseFloat(review.style.left)).toBeCloseTo(50, 3);
  });

  it('keeps non-overlapping timed events full width', () => {
    renderGrid([
      makeEvent({
        id: 'standup',
        title: 'Standup',
        interval: timedInterval('2026-11-02T09:00:00', '2026-11-02T10:00:00'),
      }),
      makeEvent({
        id: 'planning',
        title: 'Planning',
        interval: timedInterval('2026-11-02T10:00:00', '2026-11-02T11:00:00'),
      }),
    ]);

    const standup = screen.getByRole('link', { name: /Standup/ });
    const planning = screen.getByRole('link', { name: /Planning/ });

    expect(parseFloat(standup.style.width)).toBeCloseTo(100, 3);
    expect(parseFloat(standup.style.left)).toBeCloseTo(0, 3);
    expect(parseFloat(planning.style.width)).toBeCloseTo(100, 3);
    expect(parseFloat(planning.style.left)).toBeCloseTo(0, 3);
  });

  it('renders the week day headers and opens one', () => {
    const shown: { date: string | null } = { date: null };
    renderGrid([], (day) => {
      shown.date = day.toISODate();
    });

    fireEvent.click(screen.getByRole('button', { name: 'Show events on Monday, November 2' }));

    expect(shown.date).toBe('2026-11-02');
  });

  it('shows the all-day label and hour labels', () => {
    renderGrid([]);

    expect(screen.getByText('All day')).toBeDefined();
    expect(screen.getByText('12:00 AM')).toBeDefined();
    expect(screen.getByText('11:00 PM')).toBeDefined();
  });

  it('links a timed event to the editor with the query preserved', () => {
    renderGrid([
      makeEvent({
        id: 'standup',
        title: 'Standup',
        interval: timedInterval('2026-11-02T09:00:00', '2026-11-02T10:00:00'),
      }),
    ]);

    expect(screen.getByRole('link', { name: /Standup/ }).getAttribute('href')).toBe(
      '/projects/project-1/events/standup?view=week&week=2026-11-01',
    );
  });
});
