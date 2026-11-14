import { Label, Textarea } from 'flowbite-react';
import { useId } from 'react';

export type TextAreaControlProps = {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  rows?: number;
};

export function TextAreaControl(props: TextAreaControlProps) {
  const { label, value, onChange, placeholder, rows = 4 } = props;

  const id = useId();

  return (
    <Label htmlFor={id} className="flex flex-col items-start gap-1.5">
      <span>{label}</span>
      <Textarea
        id={id}
        rows={rows}
        value={value}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
      />
    </Label>
  );
}
