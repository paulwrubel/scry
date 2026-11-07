import { Button, Modal, ModalBody, ModalFooter, ModalHeader } from 'flowbite-react';
import { useState } from 'react';
import type { ProjectSnapshot } from '@/generated/ProjectSnapshot';
import { useProjectMutations } from '@/hooks/useProjectMutations';

export type DeleteProjectModalProps = {
  project: ProjectSnapshot;
  onClose: () => void;
  onDeleted: (projectId: string) => void;
};

export function DeleteProjectModal(props: DeleteProjectModalProps) {
  const { project, onClose, onDeleted } = props;
  const { deleteProject } = useProjectMutations(project);

  const [error, setError] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);

  async function handleDelete() {
    setDeleting(true);
    setError(null);

    try {
      await deleteProject();
      onDeleted(project.id);
    } catch {
      setError('Could not delete this project.');
    } finally {
      setDeleting(false);
    }
  }

  return (
    <Modal show onClose={onClose}>
      <ModalHeader>Delete project</ModalHeader>
      <ModalBody>
        <p className="text-sm">
          Delete “{project.name}”? This permanently deletes the project and all of its sections,
          tasks, and notes.
        </p>
        {error === null ? null : (
          <p className="text-danger mt-4 text-sm" role="alert">
            {error}
          </p>
        )}
      </ModalBody>
      <ModalFooter>
        <Button color="red" onClick={() => void handleDelete()} disabled={deleting}>
          {deleting ? 'Deleting...' : 'Delete project'}
        </Button>
        <Button color="alternative" onClick={onClose} disabled={deleting}>
          Cancel
        </Button>
      </ModalFooter>
    </Modal>
  );
}
