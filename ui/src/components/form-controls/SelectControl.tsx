import { HelperText, Label, Select } from 'flowbite-react';
import { useFieldContext } from '@/hooks/formContext';
import { fieldErrorMessage } from './fieldErrors';

export type SelectItem = {
  label: string;
  value: string;
};

export type SelectControlProps = {
  label: string;
  items: SelectItem[];
};

export function SelectControl(props: SelectControlProps) {
  const { label, items } = props;

  const field = useFieldContext<string>();
  const error = field.state.meta.isTouched ? fieldErrorMessage(field.state.meta.errors) : null;

  return (
    <Label className="flex w-full flex-col items-stretch gap-1.5">
      <span>{label}</span>
      <Select
        sizing="sm"
        value={field.state.value ?? ''}
        onChange={(event) => field.handleChange(event.target.value)}
      >
        {items.map((item) => (
          <option key={item.value} value={item.value}>
            {item.label}
          </option>
        ))}
      </Select>
      {error === null ? null : <HelperText color="failure">{error}</HelperText>}
    </Label>
  );
}
