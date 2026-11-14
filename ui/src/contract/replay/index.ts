import type { Change } from '../../generated/Change';
import type { Operation } from '../../generated/Operation';
import { applyChange } from './changes';
import { applyOperation } from './operations';
import type { ClientState } from './types';

export type { ClientState } from './types';

export function applyChanges(confirmed: ClientState, changes: readonly Change[]): ClientState {
  const state = structuredClone(confirmed);
  for (const change of changes) {
    applyChange(state, change);
  }
  return state;
}

export function replayOperations(
  confirmed: ClientState,
  operations: readonly Operation[],
): ClientState {
  const state = structuredClone(confirmed);
  for (const operation of operations) {
    applyOperation(state, operation);
  }
  return state;
}

export function visibleState(
  confirmed: ClientState,
  changes: readonly Change[],
  acknowledged: readonly string[],
  outbox: readonly Operation[],
): ClientState {
  const confirmedAfterPull = applyChanges(confirmed, changes);
  const acked = new Set(acknowledged);
  const remaining = outbox.filter((operation) => !acked.has(operation.operation_id));

  return replayOperations(confirmedAfterPull, remaining);
}
