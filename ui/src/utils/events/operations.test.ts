import { describe, expect, it } from 'vitest';
import type { Event } from '@/generated/Event';
import type { Interval } from '@/generated/Interval';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { createEventOperation, deleteEventOperation, updateEventOperation } from './operations';

const CREATED_AT = '2026-01-01T00:00:00Z';
const EVENT_ID = 'event-1';
const PROJECT_ID = 'project-1';

const INTERVAL: Interval = {
  start: { kind: 'date', date: '2026-11-06' },
  end: { kind: 'date', date: '2026-11-07' },
};

const NEXT_INTERVAL: Interval = {
  start: { kind: 'date', date: '2026-11-08' },
  end: { kind: 'date', date: '2026-11-09' },
};

function project(): ProjectSnapshot {
  return {
    id: PROJECT_ID,
    name: 'Project',
    description: null,
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
    sections: [],
    tasks: [],
    comments: [],
    automations: [],
    events: [],
  };
}

function event(overrides: Partial<Event> = {}): Event {
  return {
    id: EVENT_ID,
    project_id: PROJECT_ID,
    title: 'Standup',
    description: null,
    interval: INTERVAL,
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
    ...overrides,
  };
}

describe('createEventOperation', () => {
  it('builds a create_event operation with a fresh id', () => {
    const created = createEventOperation(project(), {
      title: '  Standup  ',
      description: '  ',
      interval: INTERVAL,
    });

    expect(created).not.toBeNull();
    expect(created?.operation.kind).toBe('create_event');
    expect(created?.operation.id).toBe(created?.id);
    expect(created?.operation).toMatchObject({
      project_id: PROJECT_ID,
      title: 'Standup',
      description: null,
      interval: INTERVAL,
    });
  });

  it('keeps a description', () => {
    const created = createEventOperation(project(), {
      title: 'Standup',
      description: 'Daily sync',
      interval: INTERVAL,
    });

    expect(created?.operation).toMatchObject({ description: 'Daily sync' });
  });

  it('refuses a blank title', () => {
    expect(
      createEventOperation(project(), { title: '   ', description: '', interval: INTERVAL }),
    ).toBeNull();
  });
});

describe('updateEventOperation', () => {
  it('returns nothing when no field changed', () => {
    expect(
      updateEventOperation(event(), {
        title: 'Standup',
        description: '',
        interval: INTERVAL,
      }),
    ).toBeNull();
  });

  it('sends only the title when the title changed', () => {
    const operation = updateEventOperation(event(), {
      title: 'Standup v2',
      description: '',
      interval: INTERVAL,
    });

    expect(operation).toMatchObject({
      kind: 'update_event',
      id: EVENT_ID,
      title: 'Standup v2',
      interval: null,
    });
    expect(operation).toMatchObject({ description: undefined });
  });

  it('clears a description with an empty value', () => {
    const operation = updateEventOperation(event({ description: 'Daily sync' }), {
      title: 'Standup',
      description: '',
      interval: INTERVAL,
    });

    expect(operation).toMatchObject({ description: null });
  });

  it('sends only the interval when it changed', () => {
    const operation = updateEventOperation(event(), {
      title: 'Standup',
      description: '',
      interval: NEXT_INTERVAL,
    });

    expect(operation).toMatchObject({ title: null, interval: NEXT_INTERVAL });
    expect(operation).toMatchObject({ description: undefined });
  });

  it('refuses a blank title', () => {
    expect(
      updateEventOperation(event(), { title: '  ', description: '', interval: NEXT_INTERVAL }),
    ).toBeNull();
  });
});

describe('deleteEventOperation', () => {
  it('targets the event id', () => {
    expect(deleteEventOperation(event())).toMatchObject({
      kind: 'delete_event',
      id: EVENT_ID,
    });
  });
});
