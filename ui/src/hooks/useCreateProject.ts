import { useCallback } from 'react';
import { useReplica } from '@/providers/Replica';
import type { ProjectDraft } from '@/utils/projects/operations';
import { createProjectOperation } from '@/utils/projects/operations';

export function useCreateProject() {
  const replica = useReplica();

  const createProject = useCallback(
    async (draft: ProjectDraft) => {
      const operation = createProjectOperation(draft);
      await replica.enqueue(operation);
      return operation.id;
    },
    [replica],
  );

  return { createProject };
}
