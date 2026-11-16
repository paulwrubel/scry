// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Event } from '@/generated/Event';
import type { Operation } from '@/generated/Operation';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import { DEVICE_ZONE } from '@/contract/due';
import { createReplicaMock, makeEvent, makeProject, makeReplicaSnapshot } from '@/test/fixtures';
import { EventDetail } from './EventDetail';

type RenderOptions = {
  defaultDate?: string | null;
  enqueue?: (operation: Operation) => Promise<void>;
  onSaved?: (eventId: string) => void;
  onClose?: () => void;
};

afterEach(() => {
  cleanup();
});

function renderDetail(event: Event | null, options: RenderOptions = {}) {
  const {
    defaultDate = null,
    enqueue = vi.fn(async () => undefined),
    onSaved = vi.fn(),
    onClose = vi.fn(),
  } = options;
  const project = makeProject();

  const context: ReplicaContextValue = {
    status: 'ready',
    replica: createReplicaMock(makeReplicaSnapshot([project]), { enqueue }),
    error: undefined,
    retry: () => undefined,
  };

  render(
    <ReplicaContext.Provider value={context}>
      <EventDetail
        project={project}
        event={event}
        defaultDate={defaultDate}
        onSaved={onSaved}
        onClose={onClose}
      />
    </ReplicaContext.Provider>,
  );
}

describe('EventDetail', () => {
  it('pre-fills the fields from an all-day event, showing the inclusive last day', () => {
    renderDetail(
      makeEvent({
        interval: {
          start: { kind: 'date', date: '2026-11-07' },
          end: { kind: 'date', date: '2026-11-10' },
        },
      }),
    );

    expect(screen.getByLabelText<HTMLInputElement>('Start date').value).toBe('2026-11-07');
    expect(screen.getByLabelText<HTMLInputElement>('Start time').value).toBe('');
    expect(screen.getByLabelText<HTMLInputElement>('End date').value).toBe('2026-11-09');
  });

  it('follows the start with the end until the end is edited', () => {
    renderDetail(null, { defaultDate: '2026-11-20' });

    fireEvent.change(screen.getByLabelText('Start time'), { target: { value: '09:00' } });
    expect(screen.getByLabelText<HTMLInputElement>('End date').value).toBe('2026-11-20');
    expect(screen.getByLabelText<HTMLInputElement>('End time').value).toBe('10:00');

    fireEvent.change(screen.getByLabelText('Start date'), { target: { value: '2026-11-21' } });
    expect(screen.getByLabelText<HTMLInputElement>('End date').value).toBe('2026-11-21');

    fireEvent.change(screen.getByLabelText('End time'), { target: { value: '12:00' } });
    fireEvent.change(screen.getByLabelText('Start time'), { target: { value: '08:00' } });
    expect(screen.getByLabelText<HTMLInputElement>('End time').value).toBe('12:00');
  });

  it('moves a single-day end with the start', () => {
    renderDetail(
      makeEvent({
        interval: {
          start: { kind: 'zoned', datetime: '2026-11-06T09:00:00', zone: DEVICE_ZONE },
          end: { kind: 'zoned', datetime: '2026-11-06T10:00:00', zone: DEVICE_ZONE },
        },
      }),
    );

    fireEvent.change(screen.getByLabelText('Start time'), { target: { value: '09:30' } });

    expect(screen.getByLabelText<HTMLInputElement>('End time').value).toBe('10:30');
  });

  it('leaves a multi-day end where it is when the start moves', () => {
    renderDetail(
      makeEvent({
        interval: {
          start: { kind: 'date', date: '2026-11-20' },
          end: { kind: 'date', date: '2026-11-23' },
        },
      }),
    );

    fireEvent.change(screen.getByLabelText('Start date'), { target: { value: '2026-11-21' } });

    expect(screen.getByLabelText<HTMLInputElement>('End date').value).toBe('2026-11-22');
  });

  it('saves an edited title through the outbox', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderDetail(makeEvent(), { enqueue });

    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'Standup v2' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({ kind: 'update_event', title: 'Standup v2' }),
      );
    });
  });

  it('creates an event on the given day', async () => {
    const enqueue = vi.fn(async () => undefined);
    const saved: { id: string | null } = { id: null };
    renderDetail(null, {
      defaultDate: '2026-11-20',
      enqueue,
      onSaved: (eventId) => {
        saved.id = eventId;
      },
    });

    expect(screen.getByLabelText<HTMLInputElement>('Start date').value).toBe('2026-11-20');
    expect(screen.getByLabelText<HTMLInputElement>('End date').value).toBe('2026-11-20');

    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'Offsite' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create event' }));

    await waitFor(() => {
      expect(saved.id).not.toBeNull();
    });
    expect(enqueue).toHaveBeenCalledWith(
      expect.objectContaining({ kind: 'create_event', title: 'Offsite' }),
    );
  });

  it('deletes the event and closes the panel', async () => {
    const enqueue = vi.fn(async () => undefined);
    const onClose = vi.fn();
    renderDetail(makeEvent(), { enqueue, onClose });

    fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
    fireEvent.click(screen.getByRole('button', { name: 'Delete event' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(expect.objectContaining({ kind: 'delete_event' }));
    });
    expect(onClose).toHaveBeenCalled();
  });

  it('refuses to save without a title', async () => {
    const enqueue = vi.fn(async () => undefined);
    renderDetail(makeEvent(), { enqueue });

    fireEvent.change(screen.getByLabelText('Title'), { target: { value: '' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));

    await waitFor(() => {
      expect(screen.getByText('An event needs a title.')).toBeDefined();
    });
    expect(enqueue).not.toHaveBeenCalled();
  });
});
