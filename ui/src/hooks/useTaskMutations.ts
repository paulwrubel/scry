import { useCallback } from 'react';
import type { Comment } from '@/generated/Comment';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { useReplica } from '@/providers/Replica';
import type { TaskFieldPatch } from '@/utils/taskOperations';
import {
  createCommentOperation,
  deleteCommentOperation,
  deleteTaskOperation,
  externalIdentityOperation,
  moveTaskOperation,
  nextTaskPosition,
  setTaskStateOperation,
  updateCommentOperation,
  updateTaskOperation,
} from '@/utils/taskOperations';

export function useTaskMutations(task: Task, project: ProjectSnapshot) {
  const replica = useReplica();

  const saveFields = useCallback(
    async (patch: TaskFieldPatch, sectionId: string | null) => {
      const update = updateTaskOperation(task, patch);
      if (update !== null) {
        await replica.enqueue(update);
      }

      if (sectionId !== task.section_id) {
        const move = moveTaskOperation(task, sectionId, nextTaskPosition(project.tasks, sectionId));
        if (move !== null) {
          await replica.enqueue(move);
        }
      }
    },
    [project, replica, task],
  );

  const saveExternalIdentity = useCallback(
    async (identity: ExternalIdentity | null) => {
      const operation = externalIdentityOperation(task, identity);
      if (operation !== null) {
        await replica.enqueue(operation);
      }
    },
    [replica, task],
  );

  const toggleState = useCallback(async () => {
    const operation = setTaskStateOperation(task, task.state === 'open' ? 'closed' : 'open');
    if (operation !== null) {
      await replica.enqueue(operation);
    }
  }, [replica, task]);

  const deleteTask = useCallback(async () => {
    await replica.enqueue(deleteTaskOperation(task));
  }, [replica, task]);

  const createComment = useCallback(
    async (contents: string) => {
      const operation = createCommentOperation(task, contents);
      if (operation !== null) {
        await replica.enqueue(operation);
      }
    },
    [replica, task],
  );

  const updateComment = useCallback(
    async (comment: Comment, contents: string) => {
      const operation = updateCommentOperation(comment, contents);
      if (operation !== null) {
        await replica.enqueue(operation);
      }
    },
    [replica],
  );

  const deleteComment = useCallback(
    async (comment: Comment) => {
      await replica.enqueue(deleteCommentOperation(comment));
    },
    [replica],
  );

  return {
    saveFields,
    saveExternalIdentity,
    toggleState,
    deleteTask,
    createComment,
    updateComment,
    deleteComment,
  };
}
