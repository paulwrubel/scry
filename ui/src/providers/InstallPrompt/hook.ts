import { useContext } from 'react';
import { InstallPromptContext } from './context';
import type { InstallPromptContextType } from './context';

export function useInstallPrompt(): InstallPromptContextType {
  const context = useContext(InstallPromptContext);
  if (context === null) {
    throw new Error('useInstallPrompt must be used within InstallPromptProvider');
  }
  return context;
}
