import { Alert, Card } from 'flowbite-react';
import { useState } from 'react';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { useTaskMutations } from '@/hooks/useTaskMutations';
import { DeleteTaskModal } from './DeleteTaskModal';
import { NotesSection } from './NotesSection';
import { TaskFieldsForm } from './TaskFieldsForm';
import { TaskHeader } from './TaskHeader';

type TaskDetailProps = {
  project: ProjectSnapshot;
  task: Task;
  onClose: () => void;
};

export function TaskDetail(props: TaskDetailProps) {
  const { project, task, onClose } = props;

  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { saveFields, toggleState, deleteTask, createNote, updateNote, deleteNote } =
    useTaskMutations(task, project);

  const notes = project.notes.filter((note) => note.task_id === task.id);

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
      <div className="flex flex-col gap-4 p-6">
        <TaskHeader
          project={project}
          task={task}
          onToggleState={handleToggleState}
          onRequestDelete={() => setConfirmingDelete(true)}
          onBack={onClose}
        />
        {error === null ? null : <Alert color="failure">{error}</Alert>}
        <Card>
          <TaskFieldsForm key={task.id} task={task} project={project} onSave={saveFields} />
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
