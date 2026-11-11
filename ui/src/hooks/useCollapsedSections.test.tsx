// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useCollapsedSections } from './useCollapsedSections';

afterEach(() => {
  cleanup();
  localStorage.clear();
});

describe('useCollapsedSections', () => {
  it('persists a toggle to the project key and flips membership', () => {
    const { result } = renderHook(() => useCollapsedSections('project-1'));

    act(() => {
      result.current.toggleCollapsedSection('backlog');
    });

    expect(result.current.collapsedSections.has('backlog')).toBe(true);
    expect(JSON.parse(localStorage.getItem('scry.collapsedSections.project-1') ?? 'null')).toEqual([
      'backlog',
    ]);
  });

  it('toggling twice before a commit returns the section to expanded', () => {
    const { result } = renderHook(() => useCollapsedSections('project-1'));

    act(() => {
      result.current.toggleCollapsedSection('backlog');
      result.current.toggleCollapsedSection('backlog');
    });

    expect(result.current.collapsedSections.has('backlog')).toBe(false);
  });

  it("reads the other project's stored set instead of carrying over", () => {
    localStorage.setItem('scry.collapsedSections.other', JSON.stringify(['doing']));
    const { result, rerender } = renderHook(
      ({ projectId }: { projectId: string }) => useCollapsedSections(projectId),
      { initialProps: { projectId: 'project-1' } },
    );

    act(() => {
      result.current.toggleCollapsedSection('backlog');
    });
    expect(result.current.collapsedSections.has('backlog')).toBe(true);

    rerender({ projectId: 'other' });

    expect(result.current.collapsedSections.has('backlog')).toBe(false);
    expect(result.current.collapsedSections.has('doing')).toBe(true);

    act(() => {
      result.current.toggleCollapsedSection('review');
    });

    expect(JSON.parse(localStorage.getItem('scry.collapsedSections.other') ?? 'null')).toEqual([
      'doing',
      'review',
    ]);
    expect(JSON.parse(localStorage.getItem('scry.collapsedSections.project-1') ?? 'null')).toEqual([
      'backlog',
    ]);
  });
});
