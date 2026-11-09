// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useVisualViewportInset } from './useVisualViewportInset';

const originalVisualViewport = Object.getOwnPropertyDescriptor(window, 'visualViewport');
const originalInnerHeight = Object.getOwnPropertyDescriptor(window, 'innerHeight');

function restoreProperty(target: object, key: string, descriptor: PropertyDescriptor | undefined) {
  if (descriptor === undefined) {
    Reflect.deleteProperty(target, key);
    return;
  }
  Object.defineProperty(target, key, descriptor);
}

function setInnerHeight(value: number) {
  Object.defineProperty(window, 'innerHeight', { configurable: true, value });
}

function installVisualViewport(height: number, offsetTop: number): VisualViewport {
  const viewport = new EventTarget() as unknown as VisualViewport;
  Object.defineProperty(viewport, 'height', { configurable: true, value: height });
  Object.defineProperty(viewport, 'offsetTop', { configurable: true, value: offsetTop });
  Object.defineProperty(window, 'visualViewport', { configurable: true, value: viewport });
  return viewport;
}

afterEach(() => {
  cleanup();
  restoreProperty(window, 'visualViewport', originalVisualViewport);
  restoreProperty(window, 'innerHeight', originalInnerHeight);
});

describe('useVisualViewportInset', () => {
  it('returns 0 when the visual viewport is absent', () => {
    Reflect.deleteProperty(window, 'visualViewport');

    const { result } = renderHook(() => useVisualViewportInset());

    expect(result.current).toBe(0);
  });

  it('returns 0 when the visual viewport is null', () => {
    Object.defineProperty(window, 'visualViewport', { configurable: true, value: null });

    const { result } = renderHook(() => useVisualViewportInset());

    expect(result.current).toBe(0);
  });

  it('returns the space hidden below the visual viewport', () => {
    setInnerHeight(800);
    installVisualViewport(500, 0);

    const { result } = renderHook(() => useVisualViewportInset());

    expect(result.current).toBe(300);
  });

  it('clamps to 0 when the visual viewport is taller than the layout viewport', () => {
    setInnerHeight(800);
    installVisualViewport(900, 0);

    const { result } = renderHook(() => useVisualViewportInset());

    expect(result.current).toBe(0);
  });

  it('updates on resize and scroll events', () => {
    setInnerHeight(800);
    const viewport = installVisualViewport(500, 0);

    const { result } = renderHook(() => useVisualViewportInset());
    expect(result.current).toBe(300);

    act(() => {
      Object.defineProperty(viewport, 'height', { configurable: true, value: 600 });
      viewport.dispatchEvent(new Event('resize'));
    });
    expect(result.current).toBe(200);

    act(() => {
      Object.defineProperty(viewport, 'offsetTop', { configurable: true, value: 100 });
      viewport.dispatchEvent(new Event('scroll'));
    });
    expect(result.current).toBe(100);
  });
});
