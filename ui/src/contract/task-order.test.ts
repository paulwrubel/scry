import { describe, expect, it } from 'vitest';
import type { Task } from '@/generated/Task';
import type { When } from '@/generated/When';
import { compareTasks } from './task-order';

const FIRST_ID = 'e0000000-0000-4000-8000-000000000001';
const SECOND_ID = 'e0000000-0000-4000-8000-000000000002';
const ZONE = 'UTC';

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
    due: null,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    external_identity: null,
    external_metadata: null,
    ...overrides,
  };
}

function dateWhen(date: string): When {
  return { kind: 'date', date };
}

describe('compareTasks', () => {
  it('orders manual tasks by position', () => {
    const first = task({ position: 'a0' });
    const second = task({ id: SECOND_ID, position: 'a1' });

    expect(compareTasks('manual', first, second, ZONE)).toBeLessThan(0);
    expect(compareTasks('manual', second, first, ZONE)).toBeGreaterThan(0);
  });

  it('orders priority tasks by rank', () => {
    const critical = task({ priority: 'critical' });
    const normal = task({ id: SECOND_ID, priority: 'normal' });

    expect(compareTasks('priority', critical, normal, ZONE)).toBeLessThan(0);
    expect(compareTasks('priority', normal, critical, ZONE)).toBeGreaterThan(0);
  });

  it('orders same-priority tasks by position', () => {
    const first = task({ position: 'a0' });
    const second = task({ id: SECOND_ID, position: 'a1' });

    expect(compareTasks('priority', first, second, ZONE)).toBeLessThan(0);
    expect(compareTasks('priority', second, first, ZONE)).toBeGreaterThan(0);
  });

  it('breaks same-priority position ties by id', () => {
    const first = task();
    const second = task({ id: SECOND_ID });

    expect(compareTasks('priority', first, second, ZONE)).toBeLessThan(0);
    expect(compareTasks('priority', second, first, ZONE)).toBeGreaterThan(0);
  });

  it('breaks ties by id', () => {
    const first = task();
    const second = task({ id: SECOND_ID });

    expect(compareTasks('manual', first, second, ZONE)).toBeLessThan(0);
    expect(compareTasks('manual', second, first, ZONE)).toBeGreaterThan(0);
  });

  it('orders due tasks by deadline', () => {
    const earlier = task({ due: dateWhen('2026-11-06') });
    const later = task({ id: SECOND_ID, due: dateWhen('2026-11-07') });

    expect(compareTasks('due', earlier, later, ZONE)).toBeLessThan(0);
    expect(compareTasks('due', later, earlier, ZONE)).toBeGreaterThan(0);
  });

  it('sorts tasks without a due value last', () => {
    const dated = task({ position: 'a9', due: dateWhen('2026-11-06') });
    const undated = task({ id: SECOND_ID, position: 'a0', due: null });

    expect(compareTasks('due', dated, undated, ZONE)).toBeLessThan(0);
    expect(compareTasks('due', undated, dated, ZONE)).toBeGreaterThan(0);
  });

  it('orders two undated tasks by position', () => {
    const first = task({ position: 'a0' });
    const second = task({ id: SECOND_ID, position: 'a1' });

    expect(compareTasks('due', first, second, ZONE)).toBeLessThan(0);
    expect(compareTasks('due', second, first, ZONE)).toBeGreaterThan(0);
  });

  it('breaks equal deadlines by position', () => {
    const first = task({ position: 'a0', due: dateWhen('2026-11-06') });
    const second = task({ id: SECOND_ID, position: 'a1', due: dateWhen('2026-11-06') });

    expect(compareTasks('due', first, second, ZONE)).toBeLessThan(0);
    expect(compareTasks('due', second, first, ZONE)).toBeGreaterThan(0);
  });
});
