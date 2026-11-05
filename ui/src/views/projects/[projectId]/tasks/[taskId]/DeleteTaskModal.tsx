import { Button, Modal, ModalBody, ModalFooter, ModalHeader } from 'flowbite-react';
import type { Task } from '@/generated/Task';

export type DeleteTaskModalProps = {
  task: Task;
  show: boolean;
  onClose: () => void;
  onDelete: () => Promise<void>;
};

export function DeleteTaskModal(props: DeleteTaskModalProps) {
  const { task, show, onClose, onDelete } = props;

  return (
    <Modal show={show} onClose={onClose}>
      <ModalHeader>Delete task</ModalHeader>
      <ModalBody>
        <p className="text-sm">
          Delete “{task.title}”? This change is queued and cannot be undone.
        </p>
      </ModalBody>
      <ModalFooter>
        <Button color="failure" onClick={() => void onDelete()}>
          Delete task
        </Button>
        <Button color="alternative" onClick={onClose}>
          Cancel
        </Button>
      </ModalFooter>
    </Modal>
  );
}
