export type BackButtonProps = {
  label: string;
  onClick: () => void;
};

export function BackButton(props: BackButtonProps) {
  const { label, onClick } = props;

  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className="border-line text-muted hover:border-line-hover inline-flex h-11 w-11 items-center justify-center rounded-lg border transition-colors hover:text-zinc-200 md:h-8 md:w-8"
    >
      <svg
        aria-hidden="true"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth={1.8}
        className="h-5 w-5 md:h-4 md:w-4"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18"
        />
      </svg>
    </button>
  );
}
