import { HiPencil } from 'react-icons/hi2';

export type PencilButtonProps = {
  label: string;
  onClick: () => void;
};

export function PencilButton(props: PencilButtonProps) {
  const { label, onClick } = props;

  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className="text-muted inline-flex h-6 w-6 shrink-0 items-center justify-center border-0 bg-transparent p-0 transition-colors hover:text-zinc-200 focus:outline-none focus-visible:outline-none"
    >
      <HiPencil aria-hidden="true" className="h-3.5 w-3.5" />
    </button>
  );
}
