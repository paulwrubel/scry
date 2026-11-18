import { Label, Select } from 'flowbite-react';
import { APPLY_TO_OPTIONS } from './values';
import type { ApplyTo } from './values';

export type OccurrenceModeFieldProps = {
  value: ApplyTo;
  onChange: (value: ApplyTo) => void;
};

export function OccurrenceModeField(props: OccurrenceModeFieldProps) {
  const { value, onChange } = props;

  return (
    <Label className="flex flex-col items-stretch gap-1.5">
      <span>Apply changes to</span>
      <Select
        aria-label="Apply changes to"
        value={value}
        onChange={(changeEvent) => {
          const option = APPLY_TO_OPTIONS.find(
            (candidate) => candidate.value === changeEvent.target.value,
          );
          if (option !== undefined) {
            onChange(option.value);
          }
        }}
      >
        {APPLY_TO_OPTIONS.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </Select>
    </Label>
  );
}
