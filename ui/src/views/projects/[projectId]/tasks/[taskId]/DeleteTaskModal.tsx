import { Button, Modal, ModalBody, ModalFooter, ModalHeader } from 'flowbite-react';
import { TitleText } from '@/components/TitleText';
import type { Task } from '@/generated/Task';
import { useCloseRequest } from '@/hooks/useCloseRequest';

export type DeleteTaskModalProps = {
  task: Task;
  show: boolean;
  onClose: () => void;
  onDelete: () => Promise<void>;
};

export function DeleteTaskModal(props: DeleteTaskModalProps) {
  const { task, show, onClose, onDelete } = props;

  useCloseRequest(show, onClose);

  return (
    <Modal show={show} onClose={onClose}>
      <ModalHeader>Delete task</ModalHeader>
      <ModalBody>
        <p className="text-sm">
          Delete “<TitleText title={task.title} />
          ”? This permanently deletes the task and its comments.
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
