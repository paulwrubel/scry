import { useCallback } from 'react';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { useReplica } from '@/providers/Replica';
import type { ProjectFieldPatch } from '@/utils/projectOperations';
import { deleteProjectOperation, updateProjectOperation } from '@/utils/projectOperations';

export function useProjectMutations(project: ProjectSnapshot) {
  const replica = useReplica();

  const saveFields = useCallback(
    async (patch: ProjectFieldPatch) => {
      const operation = updateProjectOperation(project, patch);
      if (operation !== null) {
        await replica.enqueue(operation);
      }
    },
    [project, replica],
  );

  const deleteProject = useCallback(async () => {
    await replica.enqueue(deleteProjectOperation(project));
  }, [project, replica]);

  return { saveFields, deleteProject };
}
