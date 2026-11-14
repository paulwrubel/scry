import { createRequiredContext } from '@/hooks/createRequiredContext';

export type InstallPromptContextType = {
  canInstall: boolean;
  isInstalled: boolean;
  isIos: boolean;
  isMobile: boolean;
  promptInstall: () => Promise<void>;
};

export const [InstallPromptContext, useInstallPrompt] =
  createRequiredContext<InstallPromptContextType>('InstallPrompt');
