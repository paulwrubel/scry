import { Button, Modal, ModalBody, ModalFooter, ModalHeader } from 'flowbite-react';
import { TitleText } from '@/components/TitleText';
import type { Event } from '@/generated/Event';
import type { When } from '@/generated/When';
import { useCloseRequest } from '@/hooks/useCloseRequest';

export type DeleteEventModalProps = {
  event: Event;
  occurrence?: When | null;
  show: boolean;
  onClose: () => void;
  onDelete: () => Promise<void>;
  onDeleteOccurrence?: () => Promise<void>;
};

export function DeleteEventModal(props: DeleteEventModalProps) {
  const { event, occurrence = null, show, onClose, onDelete, onDeleteOccurrence } = props;

  useCloseRequest(show, onClose);

  const deletingOccurrence =
    event.recurrence !== null && event.recurrence !== undefined && occurrence !== null;

  return (
    <Modal show={show} onClose={onClose} dismissible>
      <ModalHeader>Delete event</ModalHeader>
      <ModalBody>
        {deletingOccurrence ? (
          <p className="text-sm">
            Delete this occurrence of “<TitleText title={event.title} />
            ”? Only this date is removed; the rest of the series stays.
          </p>
        ) : (
          <p className="text-sm">
            Delete “<TitleText title={event.title} />
            ”? This permanently deletes the event.
          </p>
        )}
      </ModalBody>
      <ModalFooter className="justify-between">
        <Button color="alternative" onClick={onClose}>
          Cancel
        </Button>
        {deletingOccurrence ? (
          <div className="flex flex-wrap items-center justify-end gap-2">
            <Button color="red" onClick={() => void onDeleteOccurrence?.()}>
              Delete this occurrence
            </Button>
            <Button color="red" onClick={() => void onDelete()}>
              Delete series
            </Button>
          </div>
        ) : (
          <Button color="red" onClick={() => void onDelete()}>
            Delete event
          </Button>
        )}
      </ModalFooter>
    </Modal>
  );
}
