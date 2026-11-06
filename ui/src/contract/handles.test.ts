import { describe, expect, it } from 'vitest';
import { HandleError, normalizeHandle, resolveHandle, shortestHandles } from './handles';

const IDS = [
  '4f3a2b1c-9d2e-4f5a-8b6c-1d2e3f4a5b6c',
  '4f3a2b1c-9d2e-4f5a-8b6c-1d2e3f4a5b6d',
  'ffffffff-ffff-4fff-8fff-ffffffffffff',
];

function handleError(value: string): HandleError {
  try {
    normalizeHandle(value);
  } catch (error) {
    expect(error).toBeInstanceOf(HandleError);
    return error as HandleError;
  }
  throw new Error(`expected HandleError for ${value}`);
}

describe('handles', () => {
  it('normalizes a compact uppercase uuid', () => {
    expect(normalizeHandle('4F3A2B1C9D2E4F5A8B6C1D2E3F4A5B6C')).toBe(
      '4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6c',
    );
  });

  it('normalizes a canonical uuid', () => {
    expect(normalizeHandle('4F3A2B1C-9D2E-4F5A-8B6C-1D2E3F4A5B6C')).toBe(
      '4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6c',
    );
  });

  it('rejects a handle that is too short', () => {
    expect(handleError('abc').code).toBe('too_short');
  });

  it('rejects a handle with invalid characters', () => {
    expect(handleError('zzzz').code).toBe('invalid_character');
  });

  it('rejects a misplaced dash', () => {
    expect(handleError('4f3a-2b1c').code).toBe('misplaced_dash');
  });

  it('assigns the shortest distinguishing handles', () => {
    const handles = shortestHandles(IDS, 4);

    expect(handles.get(IDS[0])).toBe('4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6c');
    expect(handles.get(IDS[1])).toBe('4f3a2b1c9d2e4f5a8b6c1d2e3f4a5b6d');
    expect(handles.get(IDS[2])).toBe('ffff');
  });

  it('resolves an ambiguous prefix to every matching id', () => {
    expect(resolveHandle(IDS, '4f3a')).toEqual([IDS[0], IDS[1]]);
  });
});
