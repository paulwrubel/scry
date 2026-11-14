import { createFormHook } from '@tanstack/react-form';
import { FormPasswordField } from '@/components/form-controls/FormPasswordField';
import { FormTextAreaField } from '@/components/form-controls/FormTextAreaField';
import { FormTextField } from '@/components/form-controls/FormTextField';
import { fieldContext, formContext } from './formContext';

export const { useAppForm } = createFormHook({
  fieldContext,
  formContext,
  fieldComponents: {
    FormTextField,
    FormPasswordField,
    FormTextAreaField,
  },
  formComponents: {},
});
