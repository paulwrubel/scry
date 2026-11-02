import { generateKeyBetween, generateNKeysBetween } from 'fractional-indexing';

export type PositionErrorCode = 'invalid_key' | 'invalid_bounds';

export class PositionError extends Error {
  readonly code: PositionErrorCode;

  constructor(code: PositionErrorCode, message: string) {
    super(message);
    this.name = 'PositionError';
    this.code = code;
  }
}

const BASE_52_DIGITS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';

function digitValue(code: number): number | null {
  if (code >= 48 && code <= 57) {
    return code - 48;
  }
  if (code >= 65 && code <= 90) {
    return code - 65 + 10;
  }
  if (code >= 97 && code <= 122) {
    return code - 97 + 36;
  }
  return null;
}

function intDigitValue(code: number): number | null {
  if (code >= 65 && code <= 90) {
    return code - 65;
  }
  if (code >= 97 && code <= 122) {
    return code - 97 + 26;
  }
  return null;
}

function integerLengthForHead(head: number): number | null {
  const index = intDigitValue(head);
  if (index === null) {
    return null;
  }
  const half = BASE_52_DIGITS.length / 2;
  return index < half ? half - index + 1 : index - half + 2;
}

function isSmallestInteger(key: string): boolean {
  const zeroCount = BASE_52_DIGITS.length / 2;
  return (
    key.length === zeroCount + 1 && key.startsWith('A') && key.slice(1) === '0'.repeat(zeroCount)
  );
}

export function isValidPosition(key: string): boolean {
  if (key.length === 0) {
    return false;
  }
  for (let index = 0; index < key.length; index += 1) {
    if (digitValue(key.charCodeAt(index)) === null) {
      return false;
    }
  }
  if (isSmallestInteger(key)) {
    return false;
  }
  const integerLength = integerLengthForHead(key.charCodeAt(0));
  if (integerLength === null || integerLength > key.length) {
    return false;
  }
  return !key.slice(integerLength).endsWith('0');
}

function normalizeBounds(
  before: string | null,
  after: string | null,
): [string | null, string | null] {
  if (before !== null && !isValidPosition(before)) {
    throw new PositionError('invalid_key', `invalid order key: ${before}`);
  }
  if (after !== null && !isValidPosition(after)) {
    throw new PositionError('invalid_key', `invalid order key: ${after}`);
  }
  if (before !== null && after !== null) {
    if (before === after) {
      throw new PositionError('invalid_bounds', `invalid bounds: ${before} >= ${after}`);
    }
    return before > after ? [after, before] : [before, after];
  }
  return [before, after];
}

export function keyBetween(before: string | null, after: string | null): string {
  const [low, high] = normalizeBounds(before, after);
  try {
    return generateKeyBetween(low, high);
  } catch (error) {
    throw new PositionError('invalid_key', String(error));
  }
}

export function nKeysBetween(before: string | null, after: string | null, n: number): string[] {
  if (!Number.isInteger(n) || n < 0) {
    throw new PositionError('invalid_key', `invalid key count: ${n}`);
  }
  const [low, high] = normalizeBounds(before, after);
  try {
    return generateNKeysBetween(low, high, n);
  } catch (error) {
    throw new PositionError('invalid_key', String(error));
  }
}

export function orderPositions(keys: readonly string[]): string[] {
  return [...keys].sort(compareCodeUnits);
}

function compareCodeUnits(left: string, right: string): number {
  if (left < right) {
    return -1;
  }
  if (left > right) {
    return 1;
  }
  return 0;
}
