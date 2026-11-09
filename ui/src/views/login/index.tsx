import { Alert, Button, Card } from 'flowbite-react';
import { useState } from 'react';
import { Navigate, useNavigate, useSearchParams } from 'react-router-dom';
import { z } from 'zod';
import { useAppForm } from '@/hooks/useAppForm';
import { useAuth } from '@/providers/Auth';

const schema = z.object({
  password: z.string().min(1, 'Please enter the password.'),
});

function redirectTarget(value: string | null): string {
  if (value !== null && value.startsWith('/') && !value.startsWith('//')) {
    return value;
  }
  return '/';
}

export function LoginPage() {
  const { isAuthenticated, login } = useAuth();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const redirect = redirectTarget(searchParams.get('redirect'));
  const [error, setError] = useState<string | null>(null);

  const form = useAppForm({
    defaultValues: { password: '' },
    validators: { onChange: schema },
    listeners: { onChange: () => setError(null) },
    onSubmit: async ({ value }) => {
      setError(null);
      const result = await login(value.password);
      if (result.ok) {
        navigate(redirect, { replace: true });
        return;
      }
      setError(result.message);
    },
  });

  if (isAuthenticated) {
    return <Navigate to={redirect} replace />;
  }

  return (
    <div className="flex flex-1 items-center justify-center p-8">
      <Card className="w-full max-w-sm">
        <h1 className="text-2xl font-semibold">Sign in</h1>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            void form.handleSubmit();
          }}
          className="flex flex-col gap-3"
        >
          <form.AppField name="password">
            {(field) => <field.FormPasswordField label="Please enter the password" />}
          </form.AppField>
          {error === null ? null : <Alert color="failure">{error}</Alert>}
          <form.Subscribe
            selector={(state) => ({
              password: state.values.password,
              submitting: state.isSubmitting,
            })}
          >
            {({ password, submitting }) => (
              <Button type="submit" color="default" disabled={submitting || password.length === 0}>
                {submitting ? 'Signing in...' : 'Sign in'}
              </Button>
            )}
          </form.Subscribe>
        </form>
      </Card>
    </div>
  );
}
