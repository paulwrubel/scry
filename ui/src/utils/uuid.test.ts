import { afterEach, describe, expect, it, vi } from 'vitest';
import { newId } from './uuid';

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('newId', () => {
  it('uses randomUUID when the browser provides it', () => {
    const nativeId = '11111111-1111-4111-8111-111111111111';
    const randomUUID = vi.spyOn(crypto, 'randomUUID').mockReturnValue(nativeId);

    expect(newId()).toBe(nativeId);
    expect(randomUUID).toHaveBeenCalledTimes(1);
  });

  it('builds a v4 id from getRandomValues in an insecure context', () => {
    vi.stubGlobal('crypto', {
      getRandomValues: (bytes: Uint8Array) => {
        bytes.fill(0);
        return bytes;
      },
    });

    const id = newId();

    expect(id).toBe('00000000-0000-4000-8000-000000000000');
  });
});
