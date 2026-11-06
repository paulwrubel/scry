import { createFormHook } from '@tanstack/react-form';
import { SelectControl } from '@/components/form-controls/SelectControl';
import { FormTextAreaField } from '@/components/form-controls/FormTextAreaField';
import { FormTextField } from '@/components/form-controls/FormTextField';
import { ToggleControl } from '@/components/form-controls/ToggleControl';
import { fieldContext, formContext } from './formContext';

export const { useAppForm } = createFormHook({
  fieldContext,
  formContext,
  fieldComponents: {
    FormTextField,
    FormTextAreaField,
    SelectControl,
    ToggleControl,
  },
  formComponents: {},
});
