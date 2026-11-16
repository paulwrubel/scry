import { Button } from 'flowbite-react';
import { HiChevronLeft, HiChevronRight, HiPlus } from 'react-icons/hi2';
import type { CalendarView } from '@/utils/events/calendar';

const VIEW_OPTIONS: readonly { value: CalendarView; label: string }[] = [
  { value: 'month', label: 'Month' },
  { value: 'week', label: 'Week' },
];

export type CalendarHeaderProps = {
  label: string;
  view: CalendarView;
  onViewChange: (view: CalendarView) => void;
  onPrevious: () => void;
  onNext: () => void;
  onToday: () => void;
  onCreate: () => void;
};

export function CalendarHeader(props: CalendarHeaderProps) {
  const { label, view, onViewChange, onPrevious, onNext, onToday, onCreate } = props;

  const previousLabel = view === 'month' ? 'Previous month' : 'Previous week';
  const nextLabel = view === 'month' ? 'Next month' : 'Next week';

  return (
    <header className="border-line flex flex-wrap items-center gap-2 border-b px-4 py-3 md:grid md:grid-cols-[1fr_auto_1fr] md:px-6 md:py-4">
      <div className="flex items-center gap-2">
        <div
          role="group"
          aria-label="Calendar view"
          className="border-line flex overflow-hidden rounded-lg border text-sm"
        >
          {VIEW_OPTIONS.map((option) => (
            <button
              key={option.value}
              type="button"
              aria-pressed={view === option.value}
              onClick={() => onViewChange(option.value)}
              className={`px-3 py-1.5 transition-colors ${
                view === option.value ? 'bg-line text-zinc-100' : 'text-muted hover:bg-panel'
              }`}
            >
              {option.label}
            </button>
          ))}
        </div>
        <Button color="alternative" size="sm" onClick={onToday}>
          Today
        </Button>
      </div>
      <nav
        aria-label="Calendar period"
        className="order-last flex w-full items-center justify-center gap-2 md:order-0 md:w-auto"
      >
        <Button
          color="alternative"
          size="sm"
          aria-label={previousLabel}
          title={previousLabel}
          onClick={onPrevious}
        >
          <HiChevronLeft aria-hidden="true" className="h-4 w-4" />
        </Button>
        <h1
          aria-live="polite"
          className="min-w-0 flex-1 text-center text-base leading-tight font-semibold whitespace-normal md:w-88 md:flex-none md:text-lg md:whitespace-nowrap"
        >
          {label}
        </h1>
        <Button
          color="alternative"
          size="sm"
          aria-label={nextLabel}
          title={nextLabel}
          onClick={onNext}
        >
          <HiChevronRight aria-hidden="true" className="h-4 w-4" />
        </Button>
      </nav>
      <Button size="sm" className="ml-auto md:ml-0 md:justify-self-end" onClick={onCreate}>
        <HiPlus aria-hidden="true" className="mr-1 h-4 w-4" />
        New event
      </Button>
    </header>
  );
}
