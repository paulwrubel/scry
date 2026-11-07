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
          Delete “{task.title}”? This permanently deletes the task and its notes.
        </p>
      </ModalBody>
      <ModalFooter className="justify-between">
        <Button color="alternative" onClick={onClose}>
          Cancel
        </Button>
        <Button color="red" onClick={() => void onDelete()}>
          Delete task
        </Button>
      </ModalFooter>
    </Modal>
  );
}
