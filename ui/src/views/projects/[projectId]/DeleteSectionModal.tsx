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
          Delete “{sectionName}”? Tasks in this section will survive, and will be unassigned from
          this section.
        </p>
      </ModalBody>
      <ModalFooter className="justify-between">
        <Button color="alternative" onClick={onClose}>
          Cancel
        </Button>
        <Button
          color="red"
          onClick={() => {
            onDelete();
            onClose();
          }}
        >
          Delete section
        </Button>
      </ModalFooter>
    </Modal>
  );
}
