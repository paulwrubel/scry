// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { InstallPromptContext } from '@/providers/InstallPrompt/context';
import type { InstallPromptContextType } from '@/providers/InstallPrompt/context';
import { InstallSection } from './InstallSection';

afterEach(cleanup);

function renderSection(overrides: Partial<InstallPromptContextType> = {}) {
  const value: InstallPromptContextType = {
    canInstall: overrides.canInstall ?? false,
    isInstalled: overrides.isInstalled ?? false,
    isIos: overrides.isIos ?? false,
    promptInstall: overrides.promptInstall ?? (async (): Promise<void> => undefined),
  };

  render(
    <InstallPromptContext.Provider value={value}>
      <InstallSection />
    </InstallPromptContext.Provider>,
  );

  return value;
}

describe('InstallSection', () => {
  it('installs when the browser prompt is available', () => {
    const promptInstall = vi.fn(async (): Promise<void> => undefined);
    renderSection({ canInstall: true, promptInstall });

    fireEvent.click(screen.getByRole('button', { name: 'Install Scry' }));

    expect(promptInstall).toHaveBeenCalledTimes(1);
  });

  it('shows iOS instructions when only iOS is supported', () => {
    renderSection({ isIos: true });

    fireEvent.click(screen.getByRole('button', { name: 'Show install steps' }));

    expect(screen.getByText('Install on iPhone or iPad')).toBeTruthy();
    expect(screen.getByText(/Add to Home Screen/)).toBeTruthy();
  });

  it('renders nothing when the app is already installed', () => {
    renderSection({ canInstall: true, isInstalled: true, isIos: true });

    expect(screen.queryByRole('button')).toBeNull();
    expect(screen.queryByText('Install Scry')).toBeNull();
  });

  it('renders nothing when the platform is unsupported', () => {
    renderSection();

    expect(screen.queryByRole('button')).toBeNull();
    expect(screen.queryByText('Install Scry')).toBeNull();
  });
});
