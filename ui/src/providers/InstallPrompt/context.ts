import { createContext } from 'react';

export type InstallPromptContextType = {
  canInstall: boolean;
  isInstalled: boolean;
  isIos: boolean;
  promptInstall: () => Promise<void>;
};

export const InstallPromptContext = createContext<InstallPromptContextType | null>(null);
