import { Alert, Button, Modal, ModalBody, ModalFooter, ModalHeader } from 'flowbite-react';

export type DeleteSectionModalProps = {
  sectionName: string;
  blockingAutomations: readonly string[];
  show: boolean;
  onClose: () => void;
  onDelete: () => void;
};

export function DeleteSectionModal(props: DeleteSectionModalProps) {
  const { sectionName, blockingAutomations, show, onClose, onDelete } = props;
  const blocked = blockingAutomations.length > 0;

  return (
    <Modal show={show} onClose={onClose}>
      <ModalHeader>Delete section</ModalHeader>
      <ModalBody>
        <div className="flex flex-col gap-3">
          <p className="text-sm">
            Delete “{sectionName}”? Tasks in this section will survive, and will be unassigned from
            this section.
          </p>
          {blocked ? (
            <Alert color="warning">
              <span className="font-medium">This section is used by automations.</span>
              <span className="mt-1 block text-sm">
                Remove or change these rules before deleting the section.
              </span>
              <ul className="mt-2 list-disc pl-5 text-sm">
                {blockingAutomations.map((description) => (
                  <li key={description}>{description}</li>
                ))}
              </ul>
            </Alert>
          ) : null}
        </div>
      </ModalBody>
      <ModalFooter className="justify-between">
        <Button color="alternative" onClick={onClose}>
          Cancel
        </Button>
        <Button
          color="red"
          disabled={blocked}
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
