// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Event } from '@/generated/Event';
import type { Operation } from '@/generated/Operation';
import type { When } from '@/generated/When';
import { ReplicaContext } from '@/providers/Replica/context';
import type { ReplicaContextValue } from '@/providers/Replica/context';
import { DEVICE_ZONE } from '@/contract/due';
import { createReplicaMock, makeEvent, makeProject, makeReplicaSnapshot } from '@/test/fixtures';
import { EventDetail } from './EventDetail';

type RenderOptions = {
  defaultDate?: string | null;
  occurrence?: When | null;
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
    occurrence = null,
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
        occurrence={occurrence}
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

  it('disables Save again when a field returns to its original value', () => {
    renderDetail(makeEvent());

    const title = screen.getByLabelText('Title');
    fireEvent.change(title, { target: { value: 'Changed title' } });
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeTruthy();

    fireEvent.change(title, { target: { value: 'Event title' } });
    expect(screen.getByRole('button', { name: 'No changes' })).toBeTruthy();
  });

  it('rebases the dates and reset state when switching an occurrence to all events', () => {
    const occurrence: When = { kind: 'date', date: '2026-02-05' };
    renderDetail(
      makeEvent({
        interval: {
          start: { kind: 'date', date: '2026-02-01' },
          end: { kind: 'date', date: '2026-02-02' },
        },
        recurrence: { text: 'FREQ=DAILY;COUNT=5' },
      }),
      { occurrence },
    );

    expect(screen.getByLabelText<HTMLInputElement>('Start date').value).toBe('2026-02-05');
    expect(screen.getByLabelText<HTMLInputElement>('End date').value).toBe('2026-02-05');

    fireEvent.change(screen.getByRole('combobox', { name: 'Apply changes to' }), {
      target: { value: 'all' },
    });

    expect(screen.getByLabelText<HTMLInputElement>('Start date').value).toBe('2026-02-01');
    expect(screen.getByLabelText<HTMLInputElement>('End date').value).toBe('2026-02-01');
    expect(screen.getByRole('button', { name: 'No changes' })).toBeTruthy();
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

    fireEvent.click(screen.getByRole('button', { name: 'Delete event' }));

    const dialog = screen.getByRole('dialog');
    fireEvent.click(within(dialog).getByRole('button', { name: 'Delete event' }));

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

  it('saves an edited title as an override on the chosen occurrence', async () => {
    const enqueue = vi.fn(async () => undefined);
    const occurrence: When = { kind: 'date', date: '2026-11-08' };
    renderDetail(makeEvent({ recurrence: { text: 'FREQ=DAILY;COUNT=5' } }), {
      occurrence,
      enqueue,
    });

    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'Standup v2' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'update_event',
          title: null,
          overrides: expect.arrayContaining([
            expect.objectContaining({ original_start: occurrence, title: 'Standup v2' }),
          ]),
        }),
      );
    });
  });

  it('splits the series when applying to this and following', async () => {
    const operations: Operation[] = [];
    const enqueue = vi.fn(async (operation: Operation) => {
      operations.push(operation);
    });
    const saved = vi.fn();
    const occurrence: When = { kind: 'date', date: '2026-11-08' };
    renderDetail(makeEvent({ recurrence: { text: 'FREQ=DAILY;COUNT=5' } }), {
      occurrence,
      enqueue,
      onSaved: saved,
    });

    fireEvent.change(screen.getByLabelText('Apply changes to'), {
      target: { value: 'following' },
    });
    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'Standup v2' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));

    await waitFor(() => {
      expect(saved).toHaveBeenCalled();
    });
    expect(enqueue).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'update_event',
        recurrence: { text: 'FREQ=DAILY;COUNT=2' },
      }),
    );
    const created = operations.find((operation) => operation.kind === 'create_event');
    expect(created).toBeDefined();
    expect(saved).toHaveBeenCalledWith(created?.id);
  });

  it('deletes only the chosen occurrence from the panel choices', async () => {
    const enqueue = vi.fn(async () => undefined);
    const occurrence: When = { kind: 'date', date: '2026-11-08' };
    renderDetail(makeEvent({ recurrence: { text: 'FREQ=DAILY;COUNT=5' } }), {
      occurrence,
      enqueue,
    });

    fireEvent.click(screen.getByRole('button', { name: 'Delete this occurrence' }));

    const dialog = screen.getByRole('dialog');
    fireEvent.click(within(dialog).getByRole('button', { name: 'Delete' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'update_event',
          recurrence: { text: 'FREQ=DAILY;COUNT=5', exdates: [occurrence] },
        }),
      );
    });
    expect(enqueue).not.toHaveBeenCalledWith(expect.objectContaining({ kind: 'delete_event' }));
  });

  it('truncates the series from the delete modal instead of deleting it', async () => {
    const enqueue = vi.fn(async () => undefined);
    const occurrence: When = { kind: 'date', date: '2026-11-08' };
    renderDetail(makeEvent({ recurrence: { text: 'FREQ=DAILY;COUNT=5' } }), {
      occurrence,
      enqueue,
    });

    fireEvent.click(screen.getByRole('button', { name: 'Delete this and following' }));

    const dialog = screen.getByRole('dialog');
    fireEvent.click(within(dialog).getByRole('button', { name: 'Delete' }));

    await waitFor(() => {
      expect(enqueue).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'update_event',
          recurrence: { text: 'FREQ=DAILY;COUNT=2' },
        }),
      );
    });
    expect(enqueue).not.toHaveBeenCalledWith(expect.objectContaining({ kind: 'delete_event' }));
  });
});
