import { describe, expect, it } from 'vitest';
import type { Event } from '@/generated/Event';
import type { EventOverride } from '@/generated/EventOverride';
import type { When } from '@/generated/When';
import { splitAt, truncateAt } from './overrides';

const CREATED_AT = '2026-01-01T00:00:00Z';
const ZONE = 'UTC';

function event(overrides: Partial<Event> = {}): Event {
  return {
    id: 'event-1',
    project_id: 'project-1',
    title: 'Standup',
    description: null,
    interval: {
      start: { kind: 'date', date: '2026-11-06' },
      end: { kind: 'date', date: '2026-11-07' },
    },
    created_at: CREATED_AT,
    updated_at: CREATED_AT,
    ...overrides,
  };
}

const EXCLUDED: When = { kind: 'date', date: '2026-11-07' };
const SPLIT: When = { kind: 'date', date: '2026-11-08' };

describe('splitAt', () => {
  it('budgets a COUNT series against generated occurrences, including excluded ones', () => {
    const split = splitAt(
      event({ recurrence: { text: 'FREQ=DAILY;COUNT=5', exdates: [EXCLUDED] } }),
      SPLIT,
      ZONE,
    );

    expect(split).not.toBeNull();
    expect(split?.truncated).toEqual({ text: 'FREQ=DAILY;COUNT=2', exdates: [EXCLUDED] });
    expect(split?.following).toEqual({ text: 'FREQ=DAILY;COUNT=3' });
  });
});

describe('splitAt RDATEs', () => {
  it('moves RDATEs before the split to truncated and those at it to following', () => {
    const split = splitAt(
      event({
        recurrence: {
          text: 'FREQ=DAILY;COUNT=5',
          rdates: [
            { kind: 'date', date: '2026-11-07' },
            { kind: 'date', date: '2026-11-08' },
          ],
        },
      }),
      SPLIT,
      ZONE,
    );

    expect(split?.truncated).toEqual({
      text: 'FREQ=DAILY;COUNT=2',
      rdates: [{ kind: 'date', date: '2026-11-07' }],
    });
    expect(split?.following).toEqual({
      text: 'FREQ=DAILY;COUNT=3',
      rdates: [{ kind: 'date', date: '2026-11-08' }],
    });
  });
});

describe('truncateAt', () => {
  it('keeps the EXDATEs and overrides before the split', () => {
    const before: EventOverride = {
      original_start: { kind: 'date', date: '2026-11-06' },
      title: 'Edited',
    };
    const after: EventOverride = {
      original_start: { kind: 'date', date: '2026-11-09' },
      title: 'Later',
    };

    const truncated = truncateAt(
      event({
        recurrence: { text: 'FREQ=DAILY;COUNT=5', exdates: [EXCLUDED] },
        overrides: [before, after],
      }),
      SPLIT,
      ZONE,
    );

    expect(truncated).toEqual({
      recurrence: { text: 'FREQ=DAILY;COUNT=2', exdates: [EXCLUDED] },
      overrides: [before],
    });
  });
});
