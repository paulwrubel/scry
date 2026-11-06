import { TextInput } from 'flowbite-react';
import { useState } from 'react';
import { HiCheck, HiPencil } from 'react-icons/hi2';

const DEFAULT_ICON_BUTTON_CLASS =
  'text-muted inline-flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center border-0 bg-transparent p-0 transition-colors hover:text-zinc-200 focus:outline-none focus-visible:outline-none';

export type RenamableLabelProps = {
  label: string;
  onRename?: (newName: string) => void;
  labelAs?: 'span' | 'h2';
  labelClassName?: string;
  inputClassName?: string;
  iconButtonClassName?: string;
  inputAriaLabel?: string;
  afterLabel?: React.ReactNode;
};

export function RenamableLabel(props: RenamableLabelProps) {
  const {
    label,
    onRename,
    labelAs = 'span',
    labelClassName = 'min-w-0 truncate text-sm font-semibold',
    inputClassName = 'w-48',
    iconButtonClassName = DEFAULT_ICON_BUTTON_CLASS,
    inputAriaLabel = 'Name',
    afterLabel,
  } = props;

  const [isEditing, setIsEditing] = useState(false);
  const [editText, setEditText] = useState(label);

  function handleConfirm() {
    const trimmed = editText.trim();
    if (onRename === undefined || trimmed.length === 0 || trimmed === label) {
      setIsEditing(false);
      return;
    }

    onRename(trimmed);
    setIsEditing(false);
  }

  function handleStartEditing(event: React.MouseEvent) {
    event.stopPropagation();
    setEditText(label);
    setIsEditing(true);
  }

  if (isEditing) {
    return (
      <span className="inline-flex min-w-0 items-center gap-1" onClick={(e) => e.stopPropagation()}>
        <TextInput
          autoFocus
          sizing="sm"
          value={editText}
          onChange={(event) => setEditText(event.target.value)}
          onFocus={(event) => event.currentTarget.select()}
          onKeyDown={(event) => {
            event.stopPropagation();
            if (event.key === 'Enter') {
              event.preventDefault();
              handleConfirm();
            } else if (event.key === 'Escape') {
              event.preventDefault();
              setIsEditing(false);
            }
          }}
          aria-label={inputAriaLabel}
          className={inputClassName}
        />
        {afterLabel}
        <button
          type="button"
          aria-label={`Save ${label}`}
          title={`Save ${label}`}
          onClick={handleConfirm}
          className={iconButtonClassName}
        >
          <HiCheck aria-hidden="true" className="h-4 w-4" />
        </button>
      </span>
    );
  }

  return (
    <div className="flex min-w-0 items-center gap-2">
      {labelAs === 'h2' ? (
        <h2 className={labelClassName} title={label}>
          {label}
        </h2>
      ) : (
        <span className={labelClassName} title={label}>
          {label}
        </span>
      )}
      {afterLabel}
      {onRename === undefined ? null : (
        <button
          type="button"
          aria-label={`Rename ${label}`}
          title={`Rename ${label}`}
          onClick={handleStartEditing}
          className={iconButtonClassName}
        >
          <HiPencil aria-hidden="true" className="h-4 w-4" />
        </button>
      )}
    </div>
  );
}
