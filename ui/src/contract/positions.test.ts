import { describe, expect, it } from 'vitest';
import { PositionError, keyBetween, nKeysBetween, orderPositions } from './positions';

function positionError(callback: () => unknown): PositionError {
  try {
    callback();
  } catch (error) {
    expect(error).toBeInstanceOf(PositionError);
    return error as PositionError;
  }
  throw new Error('expected PositionError');
}

describe('positions', () => {
  it('generates a key with open bounds', () => {
    expect(keyBetween(null, null)).toBe('a0');
  });

  it('generates a key between a0 and a1', () => {
    expect(keyBetween('a0', 'a1')).toBe('a0V');
  });

  it('generates three keys with open bounds', () => {
    expect(nKeysBetween(null, null, 3)).toEqual(['a0', 'a1', 'a2']);
  });

  it('rejects an invalid order key', () => {
    expect(positionError(() => keyBetween('a00', null)).code).toBe('invalid_key');
  });

  it('orders positions by code unit', () => {
    expect(orderPositions(['a1', 'a0V', 'Zz', 'a0'])).toEqual(['Zz', 'a0', 'a0V', 'a1']);
  });
});
