import { Button } from 'flowbite-react';
import { HiChevronLeft, HiChevronRight, HiPlus } from 'react-icons/hi2';

export type MonthHeaderProps = {
  label: string;
  onPrevious: () => void;
  onNext: () => void;
  onToday: () => void;
  onCreate: () => void;
};

export function MonthHeader(props: MonthHeaderProps) {
  const { label, onPrevious, onNext, onToday, onCreate } = props;

  return (
    <div className="border-line flex flex-wrap items-center gap-2 border-b px-4 py-3 md:px-6 md:py-4">
      <Button color="alternative" size="sm" onClick={onToday}>
        Today
      </Button>
      <Button
        color="alternative"
        size="sm"
        aria-label="Previous month"
        title="Previous month"
        onClick={onPrevious}
      >
        <HiChevronLeft aria-hidden="true" className="h-4 w-4" />
      </Button>
      <h1 aria-live="polite" className="w-44 text-center text-lg font-semibold whitespace-nowrap">
        {label}
      </h1>
      <Button
        color="alternative"
        size="sm"
        aria-label="Next month"
        title="Next month"
        onClick={onNext}
      >
        <HiChevronRight aria-hidden="true" className="h-4 w-4" />
      </Button>
      <Button size="sm" className="ml-auto" onClick={onCreate}>
        <HiPlus aria-hidden="true" className="mr-1 h-4 w-4" />
        New event
      </Button>
    </div>
  );
}
