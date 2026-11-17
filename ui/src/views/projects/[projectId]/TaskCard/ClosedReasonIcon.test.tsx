// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { ClosedReasonIcon } from './ClosedReasonIcon';

afterEach(cleanup);

describe('ClosedReasonIcon', () => {
  it('labels each reason', () => {
    const { rerender } = render(<ClosedReasonIcon reason="completed" />);
    expect(screen.getByRole('img', { name: 'Closed as Completed' })).toBeDefined();

    rerender(<ClosedReasonIcon reason="cancelled" />);
    expect(screen.getByRole('img', { name: 'Closed as Cancelled' })).toBeDefined();

    rerender(<ClosedReasonIcon reason="skipped" />);
    expect(screen.getByRole('img', { name: 'Closed as Skipped' })).toBeDefined();
  });

  it('renders nothing without a reason', () => {
    const { container } = render(<ClosedReasonIcon reason={null} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders nothing for an undefined reason', () => {
    const { container } = render(<ClosedReasonIcon reason={undefined} />);
    expect(container.firstChild).toBeNull();
  });
});
