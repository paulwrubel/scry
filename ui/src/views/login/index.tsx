import { Alert, Button, Card, Label, TextInput } from 'flowbite-react';
import { useState } from 'react';
import { Navigate, useNavigate, useSearchParams } from 'react-router-dom';
import { useAuth } from '@/providers/Auth';

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
  const [token, setToken] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  if (isAuthenticated) {
    return <Navigate to={redirect} replace />;
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    setError(null);

    const result = await login(token);
    if (result.ok) {
      navigate(redirect, { replace: true });
      return;
    }

    setError(result.message);
    setSubmitting(false);
  }

  return (
    <div className="flex flex-1 items-center justify-center p-8">
      <Card className="w-full max-w-sm">
        <h1 className="text-2xl font-semibold">Sign in</h1>
        <p className="text-muted text-sm">
          Enter the API token configured on the server. It is exchanged for a short-lived access
          token held in memory plus an HttpOnly refresh cookie, and the API token itself is never
          stored in the browser.
        </p>
        <form onSubmit={handleSubmit} className="flex flex-col gap-3">
          <Label htmlFor="api-token">API token</Label>
          <TextInput
            id="api-token"
            type="password"
            autoComplete="current-password"
            value={token}
            onChange={(event) => setToken(event.target.value)}
            required
          />
          {error !== null ? <Alert color="failure">{error}</Alert> : null}
          <Button type="submit" color="default" disabled={submitting || token.length === 0}>
            {submitting ? 'Signing in...' : 'Sign in'}
          </Button>
        </form>
      </Card>
    </div>
  );
}
