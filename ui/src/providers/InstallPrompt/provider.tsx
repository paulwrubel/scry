import { useCallback, useEffect, useMemo, useState } from 'react';
import { InstallPromptContext } from './context';
import type { InstallPromptContextType } from './context';

type BeforeInstallPromptEvent = Event & {
  prompt: () => Promise<void>;
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed'; platform: string }>;
};

export type InstallPromptProviderProps = {
  children: React.ReactNode;
};

function detectInstalled(): boolean {
  if (typeof window === 'undefined') {
    return false;
  }

  if (
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(display-mode: standalone)').matches
  ) {
    return true;
  }

  // iOS Safari exposes navigator.standalone but the DOM lib does not declare it
  return (window.navigator as Navigator & { standalone?: boolean }).standalone === true;
}

function detectIos(): boolean {
  if (typeof window === 'undefined' || typeof window.navigator === 'undefined') {
    return false;
  }

  const { userAgent, platform, maxTouchPoints } = window.navigator;

  return /iphone|ipad|ipod/i.test(userAgent) || (platform === 'MacIntel' && maxTouchPoints > 1);
}

function detectMobile(): boolean {
  if (typeof window === 'undefined' || typeof window.navigator === 'undefined') {
    return false;
  }

  const { userAgent, platform, maxTouchPoints } = window.navigator;

  return (
    /android|iphone|ipad|ipod|mobile|tablet/i.test(userAgent) ||
    (platform === 'MacIntel' && maxTouchPoints > 1)
  );
}

export function InstallPromptProvider(props: InstallPromptProviderProps) {
  const { children } = props;

  const [deferredPrompt, setDeferredPrompt] = useState<BeforeInstallPromptEvent | null>(null);
  const [isInstalled, setIsInstalled] = useState(detectInstalled);
  const [isIos] = useState(detectIos);
  const [isMobile] = useState(detectMobile);

  useEffect(() => {
    function handleBeforeInstallPrompt(event: Event) {
      event.preventDefault();
      // the beforeinstallprompt event carries this payload but the DOM lib has no such type
      setDeferredPrompt(event as BeforeInstallPromptEvent);
    }

    function handleAppInstalled() {
      setDeferredPrompt(null);
      setIsInstalled(true);
    }

    window.addEventListener('beforeinstallprompt', handleBeforeInstallPrompt);
    window.addEventListener('appinstalled', handleAppInstalled);

    return () => {
      window.removeEventListener('beforeinstallprompt', handleBeforeInstallPrompt);
      window.removeEventListener('appinstalled', handleAppInstalled);
    };
  }, []);

  const promptInstall = useCallback(async (): Promise<void> => {
    if (deferredPrompt === null) {
      return;
    }

    try {
      await deferredPrompt.prompt();
      await deferredPrompt.userChoice;
    } finally {
      setDeferredPrompt(null);
    }
  }, [deferredPrompt]);

  const value = useMemo<InstallPromptContextType>(
    () => ({
      canInstall: deferredPrompt !== null,
      isInstalled,
      isIos,
      isMobile,
      promptInstall,
    }),
    [deferredPrompt, isInstalled, isIos, isMobile, promptInstall],
  );

  return <InstallPromptContext.Provider value={value}>{children}</InstallPromptContext.Provider>;
}
