import { HelperText, Label, TextInput } from 'flowbite-react';
import type { Ref } from 'react';
import { useFieldContext } from '@/hooks/formContext';
import { fieldErrorMessage } from './fieldErrors';

export type FormTextFieldProps = {
  label: string;
  placeholder?: string;
  type?: 'text' | 'number';
  hideLabel?: boolean;
  sizing?: 'sm' | 'md' | 'lg';
  autoFocus?: boolean;
  inputRef?: Ref<HTMLInputElement>;
};

export function FormTextField(props: FormTextFieldProps) {
  const {
    label,
    placeholder,
    type = 'text',
    hideLabel = false,
    sizing,
    autoFocus,
    inputRef,
  } = props;

  const field = useFieldContext<string>();
  const error = fieldErrorMessage(field.state.meta.errors);

  return (
    <Label className="flex w-full flex-col items-stretch gap-1.5">
      <span className={hideLabel ? 'sr-only' : undefined}>{label}</span>
      <TextInput
        ref={inputRef}
        type={type}
        sizing={sizing}
        autoFocus={autoFocus}
        value={field.state.value ?? ''}
        placeholder={placeholder}
        color={error === null ? undefined : 'failure'}
        onChange={(event) => field.handleChange(event.target.value)}
      />
      {error === null ? null : <HelperText color="failure">{error}</HelperText>}
    </Label>
  );
}
