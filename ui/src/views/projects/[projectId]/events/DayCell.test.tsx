// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { DateTime } from 'luxon';
import { afterEach, describe, expect, it } from 'vitest';
import type { Month } from '@/utils/events/calendar';
import { DayCell } from './DayCell';

const TODAY = DateTime.fromISO('2026-11-06T15:00:00Z', { setZone: true });
const MONTH: Month = { year: 2026, month: 11 };

afterEach(cleanup);

function renderDay(day: DateTime) {
  render(<DayCell day={day} month={MONTH} today={TODAY} onShowDay={() => undefined} />);
}

describe('DayCell', () => {
  it('marks the current day', () => {
    renderDay(TODAY.startOf('day'));

    expect(
      screen
        .getByRole('button', { name: 'Show events on Friday, November 6' })
        .getAttribute('aria-current'),
    ).toBe('date');
  });

  it('does not mark another day', () => {
    renderDay(TODAY.plus({ days: 1 }).startOf('day'));

    expect(
      screen
        .getByRole('button', { name: 'Show events on Saturday, November 7' })
        .getAttribute('aria-current'),
    ).toBeNull();
  });

  it('opens the day it covers', () => {
    const selected: { value: DateTime | null } = { value: null };
    render(
      <DayCell
        day={TODAY.startOf('day')}
        month={MONTH}
        today={TODAY}
        onShowDay={(value) => {
          selected.value = value;
        }}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Show events on Friday, November 6' }));

    expect(selected.value?.toISO()).toBe(TODAY.startOf('day').toISO());
  });
});
