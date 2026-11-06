import { Button, Modal, ModalBody, ModalFooter, ModalHeader } from 'flowbite-react';

export type DeleteSectionModalProps = {
  sectionName: string;
  show: boolean;
  onClose: () => void;
  onDelete: () => void;
};

export function DeleteSectionModal(props: DeleteSectionModalProps) {
  const { sectionName, show, onClose, onDelete } = props;

  return (
    <Modal show={show} onClose={onClose}>
      <ModalHeader>Delete section</ModalHeader>
      <ModalBody>
        <p className="text-sm">
          Delete “{sectionName}”? Tasks in this section will move to No section. This change is
          queued and cannot be undone.
        </p>
      </ModalBody>
      <ModalFooter>
        <Button
          color="red"
          onClick={() => {
            onDelete();
            onClose();
          }}
        >
          Delete section
        </Button>
        <Button color="alternative" onClick={onClose}>
          Cancel
        </Button>
      </ModalFooter>
    </Modal>
  );
}
