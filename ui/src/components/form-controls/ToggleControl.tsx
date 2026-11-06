import { ToggleSwitch } from 'flowbite-react';
import { useFieldContext } from '@/hooks/formContext';

export type ToggleControlProps = {
  label: string;
};

export function ToggleControl(props: ToggleControlProps) {
  const { label } = props;
  const field = useFieldContext<boolean>();

  return (
    <ToggleSwitch
      checked={field.state.value}
      label={label}
      onChange={(checked) => field.handleChange(checked)}
    />
  );
}
