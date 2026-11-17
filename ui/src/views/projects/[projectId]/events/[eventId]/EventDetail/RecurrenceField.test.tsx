// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { RecurrenceField } from './RecurrenceField';

afterEach(() => {
  cleanup();
});

describe('RecurrenceField', () => {
  it('shows a one-off event and applies a default weekly rule', async () => {
    const onChange = vi.fn();
    render(<RecurrenceField value="" startDate="2026-11-16" onChange={onChange} error={null} />);

    expect(screen.getByText('Does not repeat')).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: 'Add recurrence' }));
    fireEvent.click(screen.getByRole('button', { name: 'OK' }));

    await waitFor(() => {
      expect(onChange).toHaveBeenCalledWith('FREQ=WEEKLY;BYDAY=MO');
    });
  });

  it('closes the dialog when the backdrop is pressed', async () => {
    render(
      <RecurrenceField value="" startDate="2026-11-16" onChange={() => undefined} error={null} />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Add recurrence' }));
    expect(screen.getByRole('heading', { name: 'Recurrence' })).toBeTruthy();

    fireEvent.mouseDown(document.body);

    await waitFor(() => {
      expect(screen.queryByRole('heading', { name: 'Recurrence' })).toBeNull();
    });
  });

  it('shows the summary and read-only RRULE for a supported rule', () => {
    render(
      <RecurrenceField
        value="FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE"
        startDate="2026-11-16"
        onChange={() => undefined}
        error={null}
      />,
    );

    expect(screen.getByText('Every 2 weeks on Mon, Wed')).toBeTruthy();
    expect(screen.getByText('FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE')).toBeTruthy();
  });

  it('keeps an unsupported rule read-only but clearable', () => {
    const onChange = vi.fn();
    render(
      <RecurrenceField
        value="FREQ=MONTHLY;BYMONTHDAY=-1"
        startDate="2026-11-16"
        onChange={onChange}
        error={null}
      />,
    );

    expect(screen.getByText('Custom recurrence')).toBeTruthy();
    expect(
      screen.getByRole<HTMLButtonElement>('button', { name: 'Edit recurrence' }).disabled,
    ).toBe(true);

    fireEvent.click(screen.getByRole('button', { name: 'Clear' }));
    expect(onChange).toHaveBeenCalledWith('');
  });
});
