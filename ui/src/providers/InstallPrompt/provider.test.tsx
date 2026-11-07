// @vitest-environment jsdom
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useInstallPrompt } from './hook';
import { InstallPromptProvider } from './provider';

afterEach(cleanup);

function Probe() {
  const { canInstall, isInstalled, isIos, isMobile, promptInstall } = useInstallPrompt();

  return (
    <div>
      <span data-testid="can-install">{canInstall ? 'yes' : 'no'}</span>
      <span data-testid="is-installed">{isInstalled ? 'yes' : 'no'}</span>
      <span data-testid="is-ios">{isIos ? 'yes' : 'no'}</span>
      <span data-testid="is-mobile">{isMobile ? 'yes' : 'no'}</span>
      <button type="button" onClick={() => void promptInstall()}>
        install
      </button>
    </div>
  );
}

function renderProbe() {
  render(
    <InstallPromptProvider>
      <Probe />
    </InstallPromptProvider>,
  );
}

function createBeforeInstallPromptEvent() {
  const prompt = vi.fn(async (): Promise<void> => undefined);
  const event = Object.assign(new Event('beforeinstallprompt'), {
    prompt,
    userChoice: Promise.resolve({ outcome: 'accepted' as const, platform: 'web' }),
  });
  return { event, prompt };
}

describe('InstallPromptProvider', () => {
  it('captures the browser prompt and clears it after installing', async () => {
    renderProbe();

    expect(screen.getByTestId('can-install').textContent).toBe('no');

    const { event, prompt } = createBeforeInstallPromptEvent();
    act(() => {
      window.dispatchEvent(event);
    });

    expect(screen.getByTestId('can-install').textContent).toBe('yes');

    await act(async () => {
      screen.getByRole('button', { name: 'install' }).click();
    });

    await waitFor(() => {
      expect(prompt).toHaveBeenCalledTimes(1);
      expect(screen.getByTestId('can-install').textContent).toBe('no');
    });
  });

  it('marks the app installed when the appinstalled event fires', async () => {
    renderProbe();

    const { event } = createBeforeInstallPromptEvent();
    act(() => {
      window.dispatchEvent(event);
    });

    expect(screen.getByTestId('can-install').textContent).toBe('yes');

    await act(async () => {
      window.dispatchEvent(new Event('appinstalled'));
    });

    expect(screen.getByTestId('is-installed').textContent).toBe('yes');
    expect(screen.getByTestId('can-install').textContent).toBe('no');
  });

  it('detects iOS from the user agent', () => {
    const originalUserAgent = navigator.userAgent;

    Object.defineProperty(navigator, 'userAgent', {
      configurable: true,
      value: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15',
    });

    try {
      renderProbe();
      expect(screen.getByTestId('is-ios').textContent).toBe('yes');
      expect(screen.getByTestId('is-mobile').textContent).toBe('yes');
    } finally {
      Object.defineProperty(navigator, 'userAgent', {
        configurable: true,
        value: originalUserAgent,
      });
    }
  });

  it('detects Android as mobile', () => {
    const originalUserAgent = navigator.userAgent;

    Object.defineProperty(navigator, 'userAgent', {
      configurable: true,
      value: 'Mozilla/5.0 (Android 14; Mobile; rv:148.0) Gecko/148.0 Firefox/148.0',
    });

    try {
      renderProbe();
      expect(screen.getByTestId('is-mobile').textContent).toBe('yes');
      expect(screen.getByTestId('is-ios').textContent).toBe('no');
    } finally {
      Object.defineProperty(navigator, 'userAgent', {
        configurable: true,
        value: originalUserAgent,
      });
    }
  });
});
