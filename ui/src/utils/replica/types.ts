import type { ClientState } from '@/contract/op-replay';
import type { Operation } from '@/generated/Operation';

export const REPLICA_DATABASE = 'scry-replica';
export const REPLICA_DATABASE_VERSION = 1;
export const CONFIRMED_SCHEMA_VERSION = 1;

export type ReplicaOptions = {
  databaseName?: string;
  onBlocked?: () => void;
};

export type ReplicaSnapshot = {
  confirmed: ClientState;
  visible: ClientState;
  outbox: readonly Operation[];
  needsBootstrap: boolean;
  persisted: boolean;
};

export type ReplicaErrorKind = 'blocked' | 'quota' | 'storage';

export type ReplicaError = Error & { readonly kind: ReplicaErrorKind };

export function replicaError(
  kind: ReplicaErrorKind,
  message: string,
  cause?: unknown,
): ReplicaError {
  const error = new Error(message, cause === undefined ? undefined : { cause }) as ReplicaError & {
    kind: ReplicaErrorKind;
  };
  error.name = 'ReplicaError';
  error.kind = kind;
  return error;
}

export function isReplicaError(error: unknown): error is ReplicaError {
  return (
    error instanceof Error &&
    error.name === 'ReplicaError' &&
    typeof (error as { kind?: unknown }).kind === 'string'
  );
}
