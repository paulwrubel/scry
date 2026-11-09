import { HelperText, Label, TextInput } from 'flowbite-react';
import { useId, useState } from 'react';
import { HiEye, HiEyeSlash } from 'react-icons/hi2';
import { useFieldContext } from '@/hooks/formContext';
import { fieldErrorMessage } from './fieldErrors';

export type FormPasswordFieldProps = {
  label: string;
  autoComplete?: string;
  autoFocus?: boolean;
};

export function FormPasswordField(props: FormPasswordFieldProps) {
  const { label, autoComplete = 'current-password', autoFocus } = props;

  const field = useFieldContext<string>();
  const error = field.state.meta.isTouched ? fieldErrorMessage(field.state.meta.errors) : null;
  const id = useId();
  const [showPassword, setShowPassword] = useState(false);

  return (
    <div className="flex w-full flex-col gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      <TextInput
        id={id}
        type={showPassword ? 'text' : 'password'}
        autoComplete={autoComplete}
        autoFocus={autoFocus}
        value={field.state.value ?? ''}
        color={error === null ? undefined : 'failure'}
        onChange={(event) => field.handleChange(event.target.value)}
        rightIcon={() => (
          <button
            type="button"
            onClick={() => setShowPassword((visible) => !visible)}
            aria-label={showPassword ? 'Hide password' : 'Show password'}
            aria-pressed={showPassword}
            className="text-muted focus-visible:ring-primary-300 pointer-events-auto flex h-full w-8 cursor-pointer items-center justify-center border-0 bg-transparent transition-colors hover:text-zinc-200 focus:outline-none focus-visible:ring-2"
          >
            {showPassword ? (
              <HiEyeSlash className="size-5" aria-hidden="true" />
            ) : (
              <HiEye className="size-5" aria-hidden="true" />
            )}
          </button>
        )}
      />
      {error === null ? null : <HelperText color="failure">{error}</HelperText>}
    </div>
  );
}
