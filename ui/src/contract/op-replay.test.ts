import { describe, expect, it } from 'vitest';
import type { Change } from '../generated/Change';
import type { Operation } from '../generated/Operation';
import type { FixtureCase } from '../fixtures/load';
import { loadArea } from '../fixtures/load';
import type { ClientState } from './op-replay';
import { applyChanges, replayOperations, visibleState } from './op-replay';

const operations = loadArea('op-replay');

describe('op replay', () => {
  for (const testCase of operations.cases) {
    it(testCase.name, () => {
      runCase(testCase);
    });
  }
});

function runCase(testCase: FixtureCase): void {
  const confirmed = testCase.input.confirmed as unknown as ClientState;
  const ops = (testCase.input.ops ?? []) as unknown as Operation[];
  const changes = (testCase.input.changes ?? []) as unknown as Change[];
  const acked = (testCase.input.acked ?? []) as unknown as string[];

  switch (testCase.kind) {
    case 'replay':
      expect(replayOperations(confirmed, ops)).toEqual(testCase.expected.visible);
      return;
    case 'apply_changes':
      expect(applyChanges(confirmed, changes)).toEqual(testCase.expected.confirmed);
      return;
    case 'cycle':
      expect(visibleState(confirmed, changes, acked, ops)).toEqual(testCase.expected.visible);
      return;
    default:
      throw new Error(`case "${testCase.name}" uses unknown fixture kind "${testCase.kind}"`);
  }
}
