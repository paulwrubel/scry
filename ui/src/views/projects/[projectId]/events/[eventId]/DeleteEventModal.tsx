import { Button, Modal, ModalBody, ModalFooter, ModalHeader } from 'flowbite-react';
import { TitleText } from '@/components/TitleText';
import type { Event } from '@/generated/Event';
import { useCloseRequest } from '@/hooks/useCloseRequest';

export type DeleteEventModalProps = {
  event: Event;
  show: boolean;
  onClose: () => void;
  onDelete: () => Promise<void>;
};

export function DeleteEventModal(props: DeleteEventModalProps) {
  const { event, show, onClose, onDelete } = props;

  useCloseRequest(show, onClose);

  return (
    <Modal show={show} onClose={onClose}>
      <ModalHeader>Delete event</ModalHeader>
      <ModalBody>
        <p className="text-sm">
          Delete “<TitleText title={event.title} />
          ”? This permanently deletes the event.
        </p>
      </ModalBody>
      <ModalFooter className="justify-between">
        <Button color="alternative" onClick={onClose}>
          Cancel
        </Button>
        <Button color="red" onClick={() => void onDelete()}>
          Delete event
        </Button>
      </ModalFooter>
    </Modal>
  );
}
