// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { SectionColorPicker } from './SectionColorPicker';

afterEach(cleanup);

describe('SectionColorPicker', () => {
  it('opens from a hollow circle and sets a color', () => {
    const onChange = vi.fn();
    render(<SectionColorPicker color={null} onChange={onChange} />);

    fireEvent.click(screen.getByRole('button', { name: 'Set section color' }));
    fireEvent.click(screen.getByRole('button', { name: 'Blue' }));

    expect(onChange).toHaveBeenCalledWith('blue');
    expect(screen.queryByRole('button', { name: 'Blue' })).toBeNull();
  });

  it('shows the current color and clears it', () => {
    const onChange = vi.fn();
    render(<SectionColorPicker color="red" onChange={onChange} />);

    fireEvent.click(screen.getByRole('button', { name: 'Section color: Red' }));

    expect(screen.getByRole('button', { name: 'Red' }).getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByRole('button', { name: 'No color' }).getAttribute('aria-pressed')).toBe(
      'false',
    );

    fireEvent.click(screen.getByRole('button', { name: 'No color' }));

    expect(onChange).toHaveBeenCalledWith(null);
  });
});
