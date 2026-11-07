import { useCallback } from 'react';
import { useReplica } from '@/providers/Replica';
import type { ProjectDraft } from '@/utils/projectOperations';
import { createProjectOperation } from '@/utils/projectOperations';

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
