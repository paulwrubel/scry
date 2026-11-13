import { describe, expect, it } from 'vitest';
import {
  DEFAULT_TASK_SORTING_MODE,
  SHOW_PRIORITY_PARAM,
  TASK_SORTING_MODES,
  applyShowPriority,
  applyTaskSortingMode,
  parseShowPriority,
  parseTaskSortingMode,
} from './taskView';

describe('parseTaskSortingMode', () => {
  it('defaults to manual when the param is missing', () => {
    expect(parseTaskSortingMode(new URLSearchParams())).toBe('manual');
  });

  it('defaults to manual when the param is empty', () => {
    expect(parseTaskSortingMode(new URLSearchParams('sort='))).toBe('manual');
  });

  it('defaults to manual for an unknown value', () => {
    expect(parseTaskSortingMode(new URLSearchParams('sort=banana'))).toBe('manual');
  });

  it('parses every valid value', () => {
    for (const mode of TASK_SORTING_MODES) {
      expect(parseTaskSortingMode(new URLSearchParams(`sort=${mode}`))).toBe(mode);
    }
  });
});

describe('applyTaskSortingMode', () => {
  it('removes the param when applying the manual default', () => {
    const next = applyTaskSortingMode(new URLSearchParams('sort=priority'), 'manual');

    expect(next.has('sort')).toBe(false);
    expect(next.toString()).toBe('');
  });

  it('sets the param for a non-manual mode', () => {
    const next = applyTaskSortingMode(new URLSearchParams(), 'priority');

    expect(next.get('sort')).toBe('priority');
  });

  it('preserves unrelated params', () => {
    const params = new URLSearchParams('state=open&tags=bug&q=fix');

    const next = applyTaskSortingMode(params, 'priority');

    expect(next.get('state')).toBe('open');
    expect(next.get('tags')).toBe('bug');
    expect(next.get('q')).toBe('fix');
    expect(next.get('sort')).toBe('priority');
  });

  it('replaces an existing sort value without duplicating it', () => {
    const next = applyTaskSortingMode(new URLSearchParams('sort=manual'), 'priority');

    expect(next.getAll('sort')).toEqual(['priority']);
  });

  it('does not mutate the input params', () => {
    const params = new URLSearchParams('sort=priority');

    applyTaskSortingMode(params, 'manual');

    expect(params.get('sort')).toBe('priority');
  });

  it('exposes manual as the default', () => {
    expect(DEFAULT_TASK_SORTING_MODE).toBe('manual');
  });
});

describe('parseShowPriority', () => {
  it('returns false when the param is missing', () => {
    expect(parseShowPriority(new URLSearchParams())).toBe(false);
  });

  it('returns false when the param is empty', () => {
    expect(parseShowPriority(new URLSearchParams('show_priority='))).toBe(false);
  });

  it('returns false for a non-true value', () => {
    expect(parseShowPriority(new URLSearchParams('show_priority=false'))).toBe(false);
  });

  it('returns true when the param is true', () => {
    expect(parseShowPriority(new URLSearchParams('show_priority=true'))).toBe(true);
  });
});

describe('applyShowPriority', () => {
  it('removes the param when applying false', () => {
    const next = applyShowPriority(new URLSearchParams('show_priority=true'), false);

    expect(next.has(SHOW_PRIORITY_PARAM)).toBe(false);
    expect(next.toString()).toBe('');
  });

  it('sets the param when applying true', () => {
    const next = applyShowPriority(new URLSearchParams(), true);

    expect(next.get(SHOW_PRIORITY_PARAM)).toBe('true');
  });

  it('preserves unrelated params', () => {
    const params = new URLSearchParams('state=open&tags=bug&q=fix');

    const next = applyShowPriority(params, true);

    expect(next.get('state')).toBe('open');
    expect(next.get('tags')).toBe('bug');
    expect(next.get('q')).toBe('fix');
    expect(next.get(SHOW_PRIORITY_PARAM)).toBe('true');
  });

  it('replaces an existing value without duplicating it', () => {
    const next = applyShowPriority(new URLSearchParams('show_priority=true'), true);

    expect(next.getAll(SHOW_PRIORITY_PARAM)).toEqual(['true']);
  });

  it('does not mutate the input params', () => {
    const params = new URLSearchParams('show_priority=true');

    applyShowPriority(params, false);

    expect(params.get(SHOW_PRIORITY_PARAM)).toBe('true');
  });
});
