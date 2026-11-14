import type { ClientState } from '@/contract/replay';
import type { Operation } from '@/generated/Operation';

export const REPLICA_DATABASE = 'scry-replica';
export const REPLICA_DATABASE_VERSION = 3;
export const CONFIRMED_SCHEMA_VERSION = 5;

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
  lastSeenSeq: number;
};

export type ReplicaErrorKind = 'blocked' | 'quota' | 'storage';

export type ReplicaError = Error & { readonly kind: ReplicaErrorKind };

export function replicaError(
  kind: ReplicaErrorKind,
  message: string,
  cause?: unknown,
): ReplicaError {
  // TS cannot express adding fields to Error, so the cast records the shape assigned below
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
    // the error may be any Error subclass, so read the tag through a shape cast
    typeof (error as { kind?: unknown }).kind === 'string'
  );
}
