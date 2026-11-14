import { HelperText, Label, Textarea } from 'flowbite-react';
import { useFieldContext } from '@/hooks/formContext';
import { fieldErrorMessage } from './fieldErrors';

export type FormTextAreaFieldProps = {
  label: string;
  placeholder?: string;
  rows?: number;
};

export function FormTextAreaField(props: FormTextAreaFieldProps) {
  const { label, placeholder, rows = 4 } = props;

  const field = useFieldContext<string>();
  const error = field.state.meta.isTouched ? fieldErrorMessage(field.state.meta.errors) : null;

  return (
    <Label className="flex w-full flex-col items-stretch gap-1.5">
      <span>{label}</span>
      <Textarea
        rows={rows}
        value={field.state.value ?? ''}
        placeholder={placeholder}
        color={error === null ? undefined : 'failure'}
        onChange={(event) => field.handleChange(event.target.value)}
      />
      {error === null ? null : <HelperText color="failure">{error}</HelperText>}
    </Label>
  );
}
