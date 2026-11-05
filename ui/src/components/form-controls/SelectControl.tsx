import { Label, Select } from 'flowbite-react';
import { useId } from 'react';

export type SelectItem = {
  label: string;
  value: string;
};

export type SelectControlProps = {
  label: string;
  value: string;
  items: SelectItem[];
  onChange: (value: string) => void;
};

export function SelectControl(props: SelectControlProps) {
  const { label, value, items, onChange } = props;
  const id = useId();

  return (
    <Label htmlFor={id} className="flex flex-col items-start gap-1.5">
      <span>{label}</span>
      <Select id={id} value={value} onChange={(event) => onChange(event.target.value)}>
        {items.map((item) => (
          <option key={item.value} value={item.value}>
            {item.label}
          </option>
        ))}
      </Select>
    </Label>
  );
}
