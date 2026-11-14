import { describe, expect, it } from 'vitest';
import type { Task } from '../generated/Task';
import { compareTasks } from './task-order';

const FIRST_ID = 'e0000000-0000-4000-8000-000000000001';
const SECOND_ID = 'e0000000-0000-4000-8000-000000000002';

function task(overrides: Partial<Task> = {}): Task {
  return {
    id: FIRST_ID,
    project_id: '11111111-1111-4111-8111-111111111111',
    title: 'task',
    description: null,
    priority: 'normal',
    section_id: null,
    position: 'a0',
    state: 'open',
    tags: [],
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    external_identity: null,
    external_metadata: null,
    ...overrides,
  };
}

describe('compareTasks', () => {
  it('orders manual tasks by position', () => {
    const first = task({ position: 'a0' });
    const second = task({ id: SECOND_ID, position: 'a1' });

    expect(compareTasks('manual', first, second)).toBeLessThan(0);
    expect(compareTasks('manual', second, first)).toBeGreaterThan(0);
  });

  it('orders priority tasks by rank', () => {
    const critical = task({ priority: 'critical' });
    const normal = task({ id: SECOND_ID, priority: 'normal' });

    expect(compareTasks('priority', critical, normal)).toBeLessThan(0);
    expect(compareTasks('priority', normal, critical)).toBeGreaterThan(0);
  });

  it('orders same-priority tasks by position', () => {
    const first = task({ position: 'a0' });
    const second = task({ id: SECOND_ID, position: 'a1' });

    expect(compareTasks('priority', first, second)).toBeLessThan(0);
    expect(compareTasks('priority', second, first)).toBeGreaterThan(0);
  });

  it('breaks same-priority position ties by id', () => {
    const first = task();
    const second = task({ id: SECOND_ID });

    expect(compareTasks('priority', first, second)).toBeLessThan(0);
    expect(compareTasks('priority', second, first)).toBeGreaterThan(0);
  });

  it('breaks ties by id', () => {
    const first = task();
    const second = task({ id: SECOND_ID });

    expect(compareTasks('manual', first, second)).toBeLessThan(0);
    expect(compareTasks('manual', second, first)).toBeGreaterThan(0);
  });
});
