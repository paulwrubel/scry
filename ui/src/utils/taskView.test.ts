import { describe, expect, it } from 'vitest';
import {
  DEFAULT_TASK_SORTING_MODE,
  TASK_SORTING_MODES,
  applyTaskSortingMode,
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

    const next = applyTaskSortingMode(params, 'alphabetical');

    expect(next.get('state')).toBe('open');
    expect(next.get('tags')).toBe('bug');
    expect(next.get('q')).toBe('fix');
    expect(next.get('sort')).toBe('alphabetical');
  });

  it('replaces an existing sort value without duplicating it', () => {
    const next = applyTaskSortingMode(new URLSearchParams('sort=manual'), 'created');

    expect(next.getAll('sort')).toEqual(['created']);
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
