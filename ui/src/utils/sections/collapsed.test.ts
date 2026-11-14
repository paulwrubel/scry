// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { readCollapsedSections, writeCollapsedSections } from './collapsed';

afterEach(() => {
  localStorage.clear();
});

describe('collapsed sections storage', () => {
  it('returns an empty set for an unknown project', () => {
    expect(readCollapsedSections('missing').size).toBe(0);
  });

  it('round-trips written ids', () => {
    writeCollapsedSections('project-1', ['a', 'b']);

    expect([...readCollapsedSections('project-1')]).toEqual(['a', 'b']);
  });

  it('removes the key when writing an empty set', () => {
    writeCollapsedSections('project-1', ['a']);
    writeCollapsedSections('project-1', []);

    expect(localStorage.getItem('scry.collapsedSections.project-1')).toBeNull();
  });

  it('returns an empty set for malformed JSON', () => {
    localStorage.setItem('scry.collapsedSections.project-1', '{not json');

    expect(readCollapsedSections('project-1').size).toBe(0);
  });

  it('returns an empty set when the stored value is not an array', () => {
    localStorage.setItem('scry.collapsedSections.project-1', JSON.stringify({ a: 1 }));

    expect(readCollapsedSections('project-1').size).toBe(0);
  });

  it('drops non-string entries', () => {
    localStorage.setItem('scry.collapsedSections.project-1', JSON.stringify(['a', 1, null, 'b']));

    expect([...readCollapsedSections('project-1')]).toEqual(['a', 'b']);
  });
});
