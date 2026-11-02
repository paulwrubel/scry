import { describe, expect, it } from 'vitest';
import type { FixtureCase } from '../fixtures/load';
import { loadArea } from '../fixtures/load';
import { PositionError, keyBetween, nKeysBetween, orderPositions } from './positions';

const positions = loadArea('positions');

describe('positions', () => {
  for (const testCase of positions.cases) {
    it(testCase.name, () => {
      runCase(testCase);
    });
  }
});

function runCase(testCase: FixtureCase): void {
  switch (testCase.kind) {
    case 'key_between':
      runKeyBetween(testCase);
      return;
    case 'n_keys_between':
      runNKeysBetween(testCase);
      return;
    case 'narrowing':
      runNarrowing(testCase);
      return;
    case 'ordering':
      runOrdering(testCase);
      return;
    default:
      throw new Error(`case "${testCase.name}" uses unknown fixture kind "${testCase.kind}"`);
  }
}

function runKeyBetween(testCase: FixtureCase): void {
  const before = inputString(testCase.input, 'before');
  const after = inputString(testCase.input, 'after');

  if (typeof testCase.expected.key === 'string') {
    const key = keyBetween(before, after);

    expect(key).toBe(testCase.expected.key);
    assertWithin(before, after, key);
    return;
  }

  expectPositionError(() => keyBetween(before, after), expectedString(testCase, 'error'));
}

function runNKeysBetween(testCase: FixtureCase): void {
  const before = inputString(testCase.input, 'before');
  const after = inputString(testCase.input, 'after');
  const n = inputNumber(testCase.input, 'n');

  if (Array.isArray(testCase.expected.keys)) {
    const keys = nKeysBetween(before, after, n);

    expect(keys).toEqual(expectedStrings(testCase, 'keys'));
    assertStrictlyIncreasing(keys);
    for (const key of keys) {
      assertWithin(before, after, key);
    }
    return;
  }

  expectPositionError(() => nKeysBetween(before, after, n), expectedString(testCase, 'error'));
}

function runNarrowing(testCase: FixtureCase): void {
  const low = inputRequiredString(testCase.input, 'before');
  let high = inputRequiredString(testCase.input, 'after');
  const iterations = inputNumber(testCase.input, 'iterations');
  const keys: string[] = [];

  for (let index = 0; index < iterations; index += 1) {
    const next = keyBetween(low, high);
    keys.push(next);
    high = next;
  }

  expect(keys).toEqual(expectedStrings(testCase, 'keys'));
}

function runOrdering(testCase: FixtureCase): void {
  const keys = inputStrings(testCase.input, 'keys');

  expect(orderPositions(keys)).toEqual(expectedStrings(testCase, 'sorted'));
}

function inputString(input: Record<string, unknown>, field: string): string | null {
  const value = input[field];
  if (value === null || typeof value === 'string') {
    return value;
  }
  throw new Error(`fixture field "${field}" must be a string or null`);
}

function inputRequiredString(input: Record<string, unknown>, field: string): string {
  const value = inputString(input, field);
  if (value === null) {
    throw new Error(`fixture field "${field}" must be a string`);
  }
  return value;
}

function inputNumber(input: Record<string, unknown>, field: string): number {
  const value = input[field];
  if (typeof value !== 'number' || !Number.isInteger(value)) {
    throw new Error(`fixture field "${field}" must be an integer`);
  }
  return value;
}

function inputStrings(input: Record<string, unknown>, field: string): string[] {
  const value = input[field];
  if (!Array.isArray(value)) {
    throw new Error(`fixture field "${field}" must be an array`);
  }
  return value.map((item) => {
    if (typeof item !== 'string') {
      throw new Error(`fixture field "${field}" must contain only strings`);
    }
    return item;
  });
}

function expectedString(testCase: FixtureCase, field: string): string {
  const value = testCase.expected[field];
  if (typeof value !== 'string') {
    throw new Error(`case "${testCase.name}" is missing a string "${field}"`);
  }
  return value;
}

function expectedStrings(testCase: FixtureCase, field: string): string[] {
  const value = testCase.expected[field];
  if (!Array.isArray(value)) {
    throw new Error(`case "${testCase.name}" is missing an array "${field}"`);
  }
  return value.map((item) => {
    if (typeof item !== 'string') {
      throw new Error(`case "${testCase.name}" has a non-string in "${field}"`);
    }
    return item;
  });
}

function assertWithin(before: string | null, after: string | null, key: string): void {
  let low = before;
  let high = after;
  if (low !== null && high !== null && low > high) {
    [low, high] = [high, low];
  }

  if (low !== null) {
    expect(low < key).toBe(true);
  }
  if (high !== null) {
    expect(key < high).toBe(true);
  }
}

function assertStrictlyIncreasing(keys: string[]): void {
  for (let index = 1; index < keys.length; index += 1) {
    expect(keys[index - 1] < keys[index]).toBe(true);
  }
}

function expectPositionError(callback: () => unknown, code: string): void {
  try {
    callback();
  } catch (error) {
    expect(error).toBeInstanceOf(PositionError);
    expect((error as PositionError).code).toBe(code);
    return;
  }
  throw new Error(`expected PositionError(${code})`);
}
