// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { NewSectionCard } from './NewSectionCard';

afterEach(cleanup);

describe('NewSectionCard', () => {
  it('adds a trimmed section name and closes the modal', async () => {
    const onAdd = vi.fn();
    render(<NewSectionCard onAdd={onAdd} />);

    fireEvent.click(screen.getByRole('button', { name: 'Add section' }));
    fireEvent.change(screen.getByLabelText('Section name'), {
      target: { value: '  Backlog  ' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Add' }));

    await waitFor(() => {
      expect(onAdd).toHaveBeenCalledWith('Backlog');
      expect(screen.queryByRole('button', { name: 'Add' })).toBeNull();
    });
  });
});
