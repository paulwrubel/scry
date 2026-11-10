// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DeleteSectionModal } from './DeleteSectionModal';

function renderModal(blockingAutomations: readonly string[]) {
  render(
    <DeleteSectionModal
      sectionName="Todo"
      blockingAutomations={blockingAutomations}
      show
      onClose={vi.fn()}
      onDelete={vi.fn()}
    />,
  );
}

function deleteButton() {
  return screen.getByRole('button', { name: 'Delete section' }) as HTMLButtonElement;
}

afterEach(cleanup);

describe('DeleteSectionModal', () => {
  it('enables delete when no automation blocks the section', () => {
    renderModal([]);

    expect(deleteButton().disabled).toBe(false);
  });

  it('disables delete and lists the blocking rules', () => {
    renderModal(['When a task enters Todo → move to Doing']);

    expect(deleteButton().disabled).toBe(true);
    expect(screen.getByText('This section is used by automations.')).toBeDefined();
    expect(screen.getByText('When a task enters Todo → move to Doing')).toBeDefined();
  });
});
