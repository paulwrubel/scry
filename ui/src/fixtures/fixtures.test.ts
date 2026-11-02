import { describe, expect, it } from 'vitest';
import {
  FIXTURE_VERSION,
  REGISTERED_AREAS,
  fixtureAreasOnDisk,
  fixtureDir,
  loadArea,
  parseFixtureFile,
} from './load';

describe('fixture contract', () => {
  it('resolves the workspace fixtures directory', () => {
    expect(fixtureDir()).toMatch(/[\\/]fixtures$/);
  });

  it('registers every fixture directory on disk', () => {
    const registered = [...REGISTERED_AREAS].sort();

    expect(fixtureAreasOnDisk()).toEqual(registered);
  });

  it('loads every registered area', () => {
    for (const area of REGISTERED_AREAS) {
      const file = loadArea(area);

      expect(file.version).toBe(FIXTURE_VERSION);
      expect(file.area).toBe(area);
      expect(file.cases.length).toBeGreaterThan(0);
    }
  });

  it('keeps case names unique within an area', () => {
    for (const area of REGISTERED_AREAS) {
      const names = loadArea(area).cases.map((testCase) => testCase.name);

      expect(new Set(names).size).toBe(names.length);
    }
  });

  it('requires object inputs and expected values', () => {
    for (const area of REGISTERED_AREAS) {
      for (const testCase of loadArea(area).cases) {
        expect(Array.isArray(testCase.input)).toBe(false);
        expect(typeof testCase.input).toBe('object');
        expect(testCase.input).not.toBeNull();
        expect(Array.isArray(testCase.expected)).toBe(false);
        expect(typeof testCase.expected).toBe('object');
        expect(testCase.expected).not.toBeNull();
      }
    }
  });

  it('rejects unknown envelope fields', () => {
    const text = JSON.stringify({
      version: 1,
      area: 'positions',
      cases: [{ name: 'case', kind: 'insert', input: {}, expected: {} }],
      unexpected: true,
    });

    expect(() => parseFixtureFile('positions', text)).toThrow(/unknown field/);
  });

  it('rejects unknown case fields', () => {
    const text = JSON.stringify({
      version: 1,
      area: 'positions',
      cases: [
        {
          name: 'case',
          kind: 'insert',
          input: {},
          expected: {},
          unexpected: true,
        },
      ],
    });

    expect(() => parseFixtureFile('positions', text)).toThrow(/unknown field/);
  });

  it('rejects unknown versions', () => {
    const text = JSON.stringify({
      version: 2,
      area: 'positions',
      cases: [{ name: 'case', kind: 'insert', input: {}, expected: {} }],
    });

    expect(() => parseFixtureFile('positions', text)).toThrow(/version 2/);
    expect(() => parseFixtureFile('positions', text)).toThrow(/version 1/);
  });

  it('rejects a mismatched area', () => {
    const text = JSON.stringify({
      version: 1,
      area: 'positions',
      cases: [{ name: 'case', kind: 'insert', input: {}, expected: {} }],
    });

    expect(() => parseFixtureFile('handles', text)).toThrow(/"positions"/);
    expect(() => parseFixtureFile('handles', text)).toThrow(/"handles"/);
  });

  it('rejects an empty case list', () => {
    const text = JSON.stringify({ version: 1, area: 'positions', cases: [] });

    expect(() => parseFixtureFile('positions', text)).toThrow(/non-empty array/);
  });
});
