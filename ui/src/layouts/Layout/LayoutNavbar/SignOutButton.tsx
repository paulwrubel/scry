import { Button } from 'flowbite-react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '@/providers/Auth';

export function SignOutButton() {
  const { isAuthenticated, logout } = useAuth();
  const navigate = useNavigate();

  if (!isAuthenticated) {
    return null;
  }

  async function handleClick() {
    await logout();
    navigate('/login', { replace: true });
  }

  return (
    <Button size="xs" color="alternative" onClick={() => void handleClick()}>
      Sign out
    </Button>
  );
}
