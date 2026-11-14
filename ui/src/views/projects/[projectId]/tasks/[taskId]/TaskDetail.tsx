import { Alert, Card } from 'flowbite-react';
import { useState } from 'react';
import type { Comment } from '@/generated/Comment';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { Priority } from '@/generated/Priority';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import type { Task } from '@/generated/Task';
import { useEnqueue } from '@/hooks/useEnqueue';
import type { TaskFieldPatch } from '@/utils/tasks/operations';
import {
  createCommentOperation,
  deleteCommentOperation,
  deleteTaskOperation,
  externalIdentityOperation,
  setTaskStateOperation,
  taskFieldOperations,
  updateCommentOperation,
} from '@/utils/tasks/operations';
import { DeleteTaskModal } from './DeleteTaskModal';
import { ExternalLinkField } from './ExternalLinkField';
import { CommentsSection } from './CommentsSection';
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

  const enqueue = useEnqueue();

  const comments = project.comments.filter((comment) => comment.task_id === task.id);

  function patch(overrides: Partial<TaskFieldPatch>): TaskFieldPatch {
    return {
      title: task.title,
      description: task.description ?? '',
      priority: task.priority,
      tags: task.tags,
      ...overrides,
    };
  }

  async function saveFields(nextPatch: TaskFieldPatch, sectionId: string | null) {
    for (const operation of taskFieldOperations(task, nextPatch, sectionId, project.tasks)) {
      await enqueue(operation);
    }
  }

  async function saveExternalIdentity(identity: ExternalIdentity | null) {
    const operation = externalIdentityOperation(task, identity);
    if (operation !== null) {
      await enqueue(operation);
    }
  }

  async function createComment(contents: string) {
    const operation = createCommentOperation(task, contents);
    if (operation !== null) {
      await enqueue(operation);
    }
  }

  async function updateComment(comment: Comment, contents: string) {
    const operation = updateCommentOperation(comment, contents);
    if (operation !== null) {
      await enqueue(operation);
    }
  }

  async function deleteComment(comment: Comment) {
    await enqueue(deleteCommentOperation(comment));
  }

  const saveTitle = (title: string) => saveFields(patch({ title }), task.section_id);
  const saveDescription = (description: string) =>
    saveFields(patch({ description }), task.section_id);
  const savePriority = (priority: Priority) => saveFields(patch({ priority }), task.section_id);
  const saveSection = (sectionId: string | null) => saveFields(patch({}), sectionId);
  const saveTags = (tags: string[]) => saveFields(patch({ tags }), task.section_id);

  async function handleToggleState() {
    try {
      const operation = setTaskStateOperation(task, task.state === 'open' ? 'closed' : 'open');
      if (operation !== null) {
        await enqueue(operation);
      }
      setError(null);
    } catch {
      setError('Could not update this task.');
    }
  }

  async function handleDelete() {
    try {
      await enqueue(deleteTaskOperation(task));
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
          <CommentsSection
            key={task.id}
            comments={comments}
            onCreate={createComment}
            onUpdate={updateComment}
            onDelete={deleteComment}
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
