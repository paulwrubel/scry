import { useCallback, useEffect, useState } from 'react';
import { readCollapsedSections, writeCollapsedSections } from '@/utils/sections/collapsed';

type CollapsedState = { projectId: string; ids: ReadonlySet<string> };

export type UseCollapsedSectionsResult = {
  collapsedSections: ReadonlySet<string>;
  toggleCollapsedSection: (sectionId: string) => void;
};

export function useCollapsedSections(projectId: string): UseCollapsedSectionsResult {
  const [state, setState] = useState<CollapsedState>(() => ({
    projectId,
    ids: readCollapsedSections(projectId),
  }));

  // Reading during render instead of a reset effect keeps the previous
  // project's ids from being written under the new project's key.
  const ids = state.projectId === projectId ? state.ids : readCollapsedSections(projectId);

  useEffect(() => {
    if (state.projectId !== projectId) {
      return;
    }
    writeCollapsedSections(state.projectId, state.ids);
  }, [projectId, state]);

  const toggleCollapsedSection = useCallback(
    (sectionId: string) => {
      // Functional updates chain, so taps that land before React commits
      // still collapse and expand instead of repeating the first toggle.
      setState((current) => {
        const base =
          current.projectId === projectId ? current.ids : readCollapsedSections(projectId);
        const next = new Set(base);
        if (next.has(sectionId)) {
          next.delete(sectionId);
        } else {
          next.add(sectionId);
        }
        return { projectId, ids: next };
      });
    },
    [projectId],
  );

  return { collapsedSections: ids, toggleCollapsedSection };
}
