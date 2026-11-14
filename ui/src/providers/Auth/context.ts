import { createRequiredContext } from '@/hooks/createRequiredContext';

export type LoginResult = { ok: true } | { ok: false; message: string };

export type AuthContextType = {
  accessToken: string | undefined;
  isAuthenticated: boolean;
  isAuthLoading: boolean;
  authenticatedFetch: typeof fetch;
  login: (password: string) => Promise<LoginResult>;
  logout: () => Promise<void>;
};

export const [AuthContext, useAuth] = createRequiredContext<AuthContextType>('Auth');
