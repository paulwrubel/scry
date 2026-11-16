// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { DateTime } from 'luxon';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent } from '@testing-library/react';
import { makeEvent } from '@/test/fixtures';
import type { EventPlacement, WeekLane } from '@/utils/events/calendar';
import { WeekRow } from './WeekRow';

const viewport = vi.hoisted(() => ({ height: 66 }));

vi.mock('@/hooks/useElementHeight', () => ({
  useElementHeight: () => ({ ref: { current: null }, height: viewport.height }),
}));

const ZONE = 'UTC';
const WEEK = Array.from({ length: 7 }, (_, index) =>
  DateTime.fromISO('2026-11-01', { zone: ZONE }).plus({ days: index }),
);

afterEach(() => {
  viewport.height = 66;
  cleanup();
});

function placement(title: string, startIndex: number, endIndex = startIndex): EventPlacement {
  return {
    event: makeEvent({ id: title, title }),
    startIndex,
    endIndex,
    continuesBefore: false,
    continuesAfter: false,
    startTime: null,
    endTime: null,
  };
}

function lane(title: string, startIndex: number, endIndex = startIndex): WeekLane {
  return [placement(title, startIndex, endIndex)];
}

function renderWeek(lanes: WeekLane[], onShowDay: (day: DateTime) => void = () => undefined) {
  render(
    <MemoryRouter>
      <WeekRow
        week={WEEK}
        lanes={lanes}
        month={{ year: 2026, month: 11 }}
        today={DateTime.fromISO('2026-11-20', { zone: ZONE })}
        projectId="project-1"
        monthQuery="?month=2026-11"
        onShowDay={onShowDay}
      />
    </MemoryRouter>,
  );
}

describe('WeekRow overflow', () => {
  it('hides the lanes that do not fit and counts them per day', () => {
    renderWeek([lane('one', 0), lane('two', 1), lane('three', 2), lane('four', 2)]);

    expect(screen.getByText('one')).toBeDefined();
    expect(screen.getByText('two')).toBeDefined();
    expect(screen.queryByText('three')).toBeNull();
    expect(screen.queryByText('four')).toBeNull();
    expect(screen.getByText('+2 more')).toBeDefined();
  });

  it('opens the day from the overflow count', () => {
    const shown: { date: string | null } = { date: null };
    renderWeek([lane('one', 0), lane('two', 1), lane('three', 2), lane('four', 2)], (day) => {
      shown.date = day.toISODate();
    });

    fireEvent.click(screen.getByText('+2 more'));

    expect(shown.date).toBe('2026-11-03');
  });

  it('shows every lane when the week is tall enough', () => {
    viewport.height = 400;
    renderWeek([lane('one', 0), lane('two', 1), lane('three', 2)]);

    expect(screen.getByText('three')).toBeDefined();
    expect(screen.queryByText(/more/)).toBeNull();
  });
});
