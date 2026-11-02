import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export type FixtureCase = {
  name: string;
  description?: string;
  kind: string;
  input: Record<string, unknown>;
  expected: Record<string, unknown>;
};

export type FixtureFile = {
  version: number;
  description?: string;
  area: string;
  cases: FixtureCase[];
};

export const FIXTURE_VERSION = 1;
export const REGISTERED_AREAS: readonly string[] = ['positions', 'handles', 'op-replay'];

const ENVELOPE_KEYS = ['version', 'area', 'description', 'cases'] as const;
const CASE_KEYS = ['name', 'description', 'kind', 'input', 'expected'] as const;

export function fixtureDir(): string {
  return join(dirname(fileURLToPath(import.meta.url)), '../../../fixtures');
}

export function fixtureAreasOnDisk(): string[] {
  try {
    return readdirSync(fixtureDir(), { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .map((entry) => entry.name)
      .sort();
  } catch (error) {
    if (isErrnoException(error) && error.code === 'ENOENT') {
      return [];
    }
    throw error;
  }
}

function isErrnoException(error: unknown): error is { code?: string } {
  return typeof error === 'object' && error !== null && 'code' in error;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function assertExactKeys(
  value: Record<string, unknown>,
  allowed: readonly string[],
  context: string,
): void {
  for (const key of Object.keys(value)) {
    if (!allowed.includes(key)) {
      throw new Error(`${context}: unknown field "${key}"`);
    }
  }
}

function assertOptionalString(value: unknown, field: string, context: string): void {
  if (value !== undefined && typeof value !== 'string') {
    throw new Error(`${context}: "${field}" must be a string`);
  }
}

function parseCase(area: string, value: unknown, index: number, seen: Set<string>): FixtureCase {
  const context = `area "${area}" case ${index + 1}`;
  if (!isPlainObject(value)) {
    throw new Error(`${context}: case must be a JSON object`);
  }
  assertExactKeys(value, CASE_KEYS, context);

  const { name, description, kind, input, expected } = value;
  if (typeof name !== 'string' || name.length === 0) {
    throw new Error(`${context}: "name" must be a non-empty string`);
  }
  const caseContext = `${context} ("${name}")`;
  if (seen.has(name)) {
    throw new Error(`${context}: duplicate case name "${name}"`);
  }
  seen.add(name);

  assertOptionalString(description, 'description', caseContext);
  if (typeof kind !== 'string' || kind.length === 0) {
    throw new Error(`${caseContext}: "kind" must be a non-empty string`);
  }
  if (!isPlainObject(input)) {
    throw new Error(`${caseContext}: "input" must be an object`);
  }
  if (!isPlainObject(expected)) {
    throw new Error(`${caseContext}: "expected" must be an object`);
  }

  return {
    name,
    description: description as string | undefined,
    kind,
    input,
    expected,
  };
}

export function parseFixtureFile(area: string, text: string): FixtureFile {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (error) {
    throw new Error(`area "${area}": invalid fixture JSON (${String(error)})`);
  }

  if (!isPlainObject(parsed)) {
    throw new Error(`area "${area}": fixture file must be a JSON object`);
  }
  assertExactKeys(parsed, ENVELOPE_KEYS, `area "${area}"`);

  const { version, area: declaredArea, description, cases } = parsed;
  if (typeof version !== 'number' || !Number.isInteger(version)) {
    throw new Error(`area "${area}": "version" must be an integer`);
  }
  if (version !== FIXTURE_VERSION) {
    throw new Error(
      `area "${area}": unsupported fixture version ${version}; this loader supports version ${FIXTURE_VERSION}`,
    );
  }
  if (typeof declaredArea !== 'string') {
    throw new Error(`area "${area}": "area" must be a string`);
  }
  if (declaredArea !== area) {
    throw new Error(`area "${area}": fixture area mismatch: file declares "${declaredArea}"`);
  }
  assertOptionalString(description, 'description', `area "${area}"`);
  if (!Array.isArray(cases) || cases.length === 0) {
    throw new Error(`area "${area}": "cases" must be a non-empty array`);
  }

  const seen = new Set<string>();
  const parsedCases = cases.map((value, index) => parseCase(area, value, index, seen));

  return {
    version,
    area: declaredArea,
    description: description as string | undefined,
    cases: parsedCases,
  };
}

export function loadArea(area: string): FixtureFile {
  const path = join(fixtureDir(), area, 'cases.json');
  let text: string;
  try {
    text = readFileSync(path, 'utf8');
  } catch (error) {
    throw new Error(`area "${area}": failed to read ${path} (${String(error)})`);
  }
  return parseFixtureFile(area, text);
}
