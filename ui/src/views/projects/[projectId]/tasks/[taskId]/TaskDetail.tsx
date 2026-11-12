import { Alert, Card } from 'flowbite-react';
import { useState } from 'react';
import type { Priority } from '@/generated/Priority';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { useTaskMutations } from '@/hooks/useTaskMutations';
import type { TaskFieldPatch } from '@/utils/taskOperations';
import { DeleteTaskModal } from './DeleteTaskModal';
import { ExternalLinkField } from './ExternalLinkField';
import { NotesSection } from './NotesSection';
import { TaskActions } from './TaskActions';
import { TaskFields } from './TaskFields';
import { TaskHeader } from './TaskHeader';
import { TaskMeta } from './TaskMeta';

export type TaskDetailProps = {
  project: ProjectSnapshot;
  task: Task;
  handle: string;
  onClose: () => void;
};

export function TaskDetail(props: TaskDetailProps) {
  const { project, task, handle, onClose } = props;

  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const {
    saveFields,
    saveExternalIdentity,
    toggleState,
    deleteTask,
    createNote,
    updateNote,
    deleteNote,
  } = useTaskMutations(task, project);

  const notes = project.notes.filter((note) => note.task_id === task.id);

  function patch(overrides: Partial<TaskFieldPatch>): TaskFieldPatch {
    return {
      title: task.title,
      description: task.description ?? '',
      priority: task.priority,
      tags: task.tags,
      ...overrides,
    };
  }

  const saveTitle = (title: string) => saveFields(patch({ title }), task.section_id);
  const saveDescription = (description: string) =>
    saveFields(patch({ description }), task.section_id);
  const savePriority = (priority: Priority) => saveFields(patch({ priority }), task.section_id);
  const saveSection = (sectionId: string | null) => saveFields(patch({}), sectionId);
  const saveTags = (tags: string[]) => saveFields(patch({ tags }), task.section_id);

  async function handleToggleState() {
    try {
      await toggleState();
      setError(null);
    } catch {
      setError('Could not update this task.');
    }
  }

  async function handleDelete() {
    try {
      await deleteTask();
      setConfirmingDelete(false);
      onClose();
    } catch {
      setError('Could not delete this task.');
    }
  }

  return (
    <>
      <div className="flex min-h-full flex-col gap-4 p-6">
        <TaskHeader
          project={project}
          task={task}
          handle={handle}
          onSaveTitle={saveTitle}
          onBack={onClose}
        />
        <TaskMeta
          task={task}
          project={project}
          onSavePriority={savePriority}
          onSaveSection={saveSection}
        />
        <ExternalLinkField task={task} onSave={saveExternalIdentity} />
        {error === null ? null : <Alert color="failure">{error}</Alert>}
        <Card>
          <TaskFields task={task} onSaveDescription={saveDescription} onSaveTags={saveTags} />
        </Card>
        <Card>
          <NotesSection
            key={task.id}
            notes={notes}
            onCreate={createNote}
            onUpdate={updateNote}
            onDelete={deleteNote}
          />
        </Card>
        <div className="border-line mt-auto flex flex-wrap items-center gap-2 border-t pt-4">
          <TaskActions
            task={task}
            onToggleState={handleToggleState}
            onRequestDelete={() => setConfirmingDelete(true)}
          />
        </div>
      </div>
      <DeleteTaskModal
        task={task}
        show={confirmingDelete}
        onClose={() => setConfirmingDelete(false)}
        onDelete={handleDelete}
      />
    </>
  );
}
