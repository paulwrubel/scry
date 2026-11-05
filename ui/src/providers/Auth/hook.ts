import { useContext } from 'react';
import { AuthContext } from './context';
import type { AuthContextType } from './context';

export function useAuth(): AuthContextType {
  const context = useContext(AuthContext);
  if (context === null) {
    throw new Error('useAuth must be used within AuthProvider');
  }
  return context;
}
