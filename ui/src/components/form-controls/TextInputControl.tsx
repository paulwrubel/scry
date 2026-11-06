import { Label, TextInput } from 'flowbite-react';
import { useId } from 'react';

export type TextInputControlProps = {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
};

export function TextInputControl(props: TextInputControlProps) {
  const { label, value, onChange, placeholder } = props;
  const id = useId();

  return (
    <Label htmlFor={id} className="flex w-full flex-col items-stretch gap-1.5">
      <span>{label}</span>
      <TextInput
        id={id}
        value={value}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
      />
    </Label>
  );
}
