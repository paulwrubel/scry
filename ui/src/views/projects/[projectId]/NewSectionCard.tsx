import {
  Button,
  Card,
  Modal,
  ModalBody,
  ModalFooter,
  ModalHeader,
  TextInput,
} from 'flowbite-react';
import type { CardTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { useState } from 'react';
import { HiPlusCircle } from 'react-icons/hi2';

const cardTheme: DeepPartial<CardTheme> = {
  root: {
    base: 'flex h-full rounded-lg border-2 border-dashed border-line-strong bg-transparent shadow-none',
    children: 'flex h-full flex-col p-0',
  },
};

export type NewSectionCardProps = {
  onAdd: (name: string) => void;
};

export function NewSectionCard(props: NewSectionCardProps) {
  const { onAdd } = props;

  const [open, setOpen] = useState(false);
  const [name, setName] = useState('');

  function close() {
    setName('');
    setOpen(false);
  }

  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = name.trim();
    if (trimmed.length === 0) {
      return;
    }

    onAdd(trimmed);
    close();
  }

  return (
    <>
      <Card
        theme={cardTheme}
        className="h-32 w-full shrink-0 bg-transparent md:h-full md:w-96 dark:bg-transparent"
      >
        <button
          type="button"
          onClick={() => setOpen(true)}
          className="text-muted flex h-full w-full flex-col items-center justify-center gap-2 rounded-lg bg-transparent transition-colors hover:text-zinc-200 dark:bg-transparent"
        >
          <HiPlusCircle aria-hidden="true" className="h-8 w-8" />
          <span className="text-base font-normal italic">Add section</span>
        </button>
      </Card>
      <Modal show={open} onClose={close}>
        <ModalHeader>Add section</ModalHeader>
        <ModalBody>
          <form id="new-section-form" onSubmit={handleSubmit} className="flex flex-col gap-2">
            <TextInput
              autoFocus
              sizing="sm"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="Section name"
              aria-label="Section name"
            />
          </form>
        </ModalBody>
        <ModalFooter>
          <Button color="default" onClick={close}>
            Cancel
          </Button>
          <Button type="submit" form="new-section-form" disabled={name.trim().length === 0}>
            Add
          </Button>
        </ModalFooter>
      </Modal>
    </>
  );
}
