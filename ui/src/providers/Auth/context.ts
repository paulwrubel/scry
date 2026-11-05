import { createContext } from 'react';

export type LoginResult = { ok: true } | { ok: false; message: string };

export type AuthContextType = {
  isAuthenticated: boolean;
  isAuthLoading: boolean;
  login: (token: string) => Promise<LoginResult>;
  logout: () => Promise<void>;
};

export const AuthContext = createContext<AuthContextType | null>(null);
