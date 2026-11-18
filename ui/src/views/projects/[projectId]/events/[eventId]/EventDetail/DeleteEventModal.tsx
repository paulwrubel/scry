import { useState } from 'react';
import { Button, Modal, ModalBody, ModalFooter, ModalHeader } from 'flowbite-react';
import { TitleText } from '@/components/TitleText';
import type { Event } from '@/generated/Event';
import { useCloseRequest } from '@/hooks/useCloseRequest';

export type DeleteEventMode = 'occurrence' | 'following' | 'series';

export type DeleteEventModalProps = {
  show: boolean;
  event: Event;
  mode: DeleteEventMode;
  onConfirm: () => Promise<void>;
  onClose: () => void;
};

export function DeleteEventModal(props: DeleteEventModalProps) {
  const { show, event, mode, onConfirm, onClose } = props;

  const [deleting, setDeleting] = useState(false);

  useCloseRequest(show, onClose);

  const recurring = event.recurrence !== null && event.recurrence !== undefined;

  async function confirm() {
    setDeleting(true);
    try {
      await onConfirm();
    } finally {
      setDeleting(false);
    }
  }

  const heading =
    mode === 'occurrence'
      ? 'Delete this occurrence'
      : mode === 'following'
        ? 'Delete this and following'
        : recurring
          ? 'Delete series'
          : 'Delete event';

  return (
    <Modal show={show} onClose={onClose} dismissible>
      <ModalHeader>{heading}</ModalHeader>
      <ModalBody>
        <p className="text-sm">
          {mode === 'occurrence' ? (
            <>
              Delete “<TitleText title={event.title} />” on this date?
            </>
          ) : mode === 'following' ? (
            <>
              Delete “<TitleText title={event.title} />” from this date onwards?
            </>
          ) : (
            <>
              Delete “<TitleText title={event.title} />”{'?'}
            </>
          )}
        </p>
        <p className="text-muted mt-3 text-sm">
          {mode === 'occurrence'
            ? 'Only this date is removed. The rest of the series stays.'
            : mode === 'following'
              ? 'This date and every later occurrence are removed.'
              : recurring
                ? 'This permanently deletes the event and every one of its occurrences.'
                : 'This permanently deletes the event.'}
        </p>
      </ModalBody>
      <ModalFooter className="justify-between">
        <Button color="alternative" onClick={onClose} disabled={deleting}>
          Keep
        </Button>
        <Button color="red" onClick={() => void confirm()} disabled={deleting}>
          {deleting ? 'Deleting...' : mode === 'series' && !recurring ? 'Delete event' : 'Delete'}
        </Button>
      </ModalFooter>
    </Modal>
  );
}
