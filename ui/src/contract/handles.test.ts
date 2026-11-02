import { describe, expect, it } from 'vitest';
import type { FixtureCase } from '../fixtures/load';
import { loadArea } from '../fixtures/load';
import { HandleError, normalizeHandle, resolveHandle, shortestHandles } from './handles';

const handles = loadArea('handles');

describe('handles', () => {
  for (const testCase of handles.cases) {
    it(testCase.name, () => {
      runCase(testCase);
    });
  }
});

function runCase(testCase: FixtureCase): void {
  switch (testCase.kind) {
    case 'normalize':
      runNormalize(testCase);
      return;
    case 'shortest':
      runShortest(testCase);
      return;
    case 'resolve':
      runResolve(testCase);
      return;
    default:
      throw new Error(`case "${testCase.name}" uses unknown fixture kind "${testCase.kind}"`);
  }
}

function runNormalize(testCase: FixtureCase): void {
  const value = inputRequiredString(testCase.input, 'value');

  if (typeof testCase.expected.hex === 'string') {
    expect(normalizeHandle(value)).toBe(testCase.expected.hex);
    return;
  }

  expectHandleError(() => normalizeHandle(value), expectedString(testCase, 'error'));
}

function runShortest(testCase: FixtureCase): void {
  const ids = inputStrings(testCase.input, 'ids');
  const minLength = inputNumber(testCase.input, 'min_length');
  const shortest = shortestHandles(ids, minLength);
  const actual = [...shortest.entries()]
    .map(([id, handle]) => ({ id, handle }))
    .sort((left, right) => compareCodeUnits(left.id, right.id));

  expect(actual).toEqual(expectedHandles(testCase));
}

function runResolve(testCase: FixtureCase): void {
  const ids = inputStrings(testCase.input, 'ids');
  const handle = inputRequiredString(testCase.input, 'handle');

  if (Array.isArray(testCase.expected.matches)) {
    expect(resolveHandle(ids, handle)).toEqual(expectedStrings(testCase, 'matches'));
    return;
  }

  expectHandleError(() => resolveHandle(ids, handle), expectedString(testCase, 'error'));
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

function expectedHandles(testCase: FixtureCase): { id: string; handle: string }[] {
  const value = testCase.expected.handles;
  if (!Array.isArray(value)) {
    throw new Error(`case "${testCase.name}" is missing an array "handles"`);
  }
  return value.map((item) => {
    if (typeof item !== 'object' || item === null || Array.isArray(item)) {
      throw new Error(`case "${testCase.name}" has a non-object in "handles"`);
    }
    const record = item as Record<string, unknown>;
    if (typeof record.id !== 'string' || typeof record.handle !== 'string') {
      throw new Error(`case "${testCase.name}" has an invalid handle entry`);
    }
    return { id: record.id, handle: record.handle };
  });
}

function expectHandleError(callback: () => unknown, code: string): void {
  try {
    callback();
  } catch (error) {
    expect(error).toBeInstanceOf(HandleError);
    expect((error as HandleError).code).toBe(code);
    return;
  }
  throw new Error(`expected HandleError(${code})`);
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
