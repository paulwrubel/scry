import { HiChevronDown } from 'react-icons/hi2';

const SELECT_CLASS =
  'border-line bg-panel text-zinc-200 cursor-pointer appearance-none rounded-full border py-1 pr-7 pl-7 text-xs outline-none disabled:cursor-not-allowed disabled:opacity-60';

const CHEVRON_CLASS =
  'text-muted pointer-events-none absolute top-1/2 right-2 h-3 w-3 -translate-y-1/2';

export type MetaSelectProps = {
  ariaLabel: string;
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
  dot: React.ReactNode;
  error: string | null;
  children: React.ReactNode;
};

export function MetaSelect(props: MetaSelectProps) {
  const { ariaLabel, value, disabled, onChange, dot, error, children } = props;

  return (
    <div className="flex flex-col gap-1">
      <div className="relative inline-flex">
        <select
          aria-label={ariaLabel}
          value={value}
          disabled={disabled}
          onChange={(event) => onChange(event.target.value)}
          className={SELECT_CLASS}
        >
          {children}
        </select>
        {dot}
        <HiChevronDown aria-hidden="true" className={CHEVRON_CLASS} />
      </div>
      {error === null ? null : (
        <p className="text-danger text-xs" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
