// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { When } from '@/generated/When';
import { DueDate } from './DueDate';

afterEach(cleanup);

const PAST: When = { kind: 'date', date: '2020-01-01' };
const FUTURE: When = { kind: 'date', date: '2999-01-01' };

describe('DueDate', () => {
  it('renders a due label', () => {
    render(<DueDate due={FUTURE} state="open" />);

    const label = screen.getByText(/Due/);
    expect(label.getAttribute('aria-label')).toBe(label.getAttribute('title'));
  });

  it('marks an open past due as overdue', () => {
    render(<DueDate due={PAST} state="open" />);

    expect(screen.getByText(/Due/).getAttribute('aria-label')).toMatch(/^Overdue, was due/);
  });

  it('does not mark a closed past due as overdue', () => {
    render(<DueDate due={PAST} state="closed" />);

    expect(screen.getByText(/Due/).getAttribute('aria-label')).not.toMatch(/^Overdue/);
  });

  it('renders nothing without a due date', () => {
    const { container } = render(<DueDate due={null} state="open" />);

    expect(container.firstChild).toBeNull();
  });
});
