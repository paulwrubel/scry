import { useCallback } from 'react';
import type { Note } from '@/generated/Note';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { useReplica } from '@/providers/Replica';
import type { TaskFieldPatch } from '@/utils/taskOperations';
import {
  createNoteOperation,
  deleteNoteOperation,
  deleteTaskOperation,
  moveTaskOperation,
  setTaskStateOperation,
  updateNoteOperation,
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

      const move = moveTaskOperation(task, project, sectionId);
      if (move !== null) {
        await replica.enqueue(move);
      }
    },
    [project, replica, task],
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

  const createNote = useCallback(
    async (contents: string) => {
      const operation = createNoteOperation(task, contents);
      if (operation !== null) {
        await replica.enqueue(operation);
      }
    },
    [replica, task],
  );

  const updateNote = useCallback(
    async (note: Note, contents: string) => {
      const operation = updateNoteOperation(note, contents);
      if (operation !== null) {
        await replica.enqueue(operation);
      }
    },
    [replica],
  );

  const deleteNote = useCallback(
    async (note: Note) => {
      await replica.enqueue(deleteNoteOperation(note));
    },
    [replica],
  );

  return { saveFields, toggleState, deleteTask, createNote, updateNote, deleteNote };
}
