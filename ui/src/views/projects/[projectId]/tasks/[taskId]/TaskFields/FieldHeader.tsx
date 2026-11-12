import { PencilButton } from './PencilButton';

export type FieldHeaderProps = {
  label: string;
  onEdit?: () => void;
};

export function FieldHeader(props: FieldHeaderProps) {
  const { label, onEdit } = props;

  return (
    <div className="flex items-center justify-between gap-2">
      <span className="text-muted text-sm font-semibold">{label}</span>
      {onEdit === undefined ? null : (
        <PencilButton label={`Edit ${label.toLowerCase()}`} onClick={onEdit} />
      )}
    </div>
  );
}
