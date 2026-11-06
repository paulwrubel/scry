import { HelperText, Label, TextInput } from 'flowbite-react';
import { useFieldContext } from '@/hooks/formContext';
import { fieldErrorMessage } from './fieldErrors';

export type FormTextFieldProps = {
  label: string;
  placeholder?: string;
  type?: 'text' | 'number';
};

export function FormTextField(props: FormTextFieldProps) {
  const { label, placeholder, type = 'text' } = props;
  const field = useFieldContext<string>();
  const error = fieldErrorMessage(field.state.meta.errors);

  return (
    <Label className="flex w-full flex-col items-stretch gap-1.5">
      <span>{label}</span>
      <TextInput
        type={type}
        value={field.state.value ?? ''}
        placeholder={placeholder}
        color={error === null ? undefined : 'failure'}
        onChange={(event) => field.handleChange(event.target.value)}
      />
      {error === null ? null : <HelperText color="failure">{error}</HelperText>}
    </Label>
  );
}
