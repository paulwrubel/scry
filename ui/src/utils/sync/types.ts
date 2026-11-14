import type { RejectionReason } from '@/generated/RejectionReason';
import type { Replica } from '@/utils/replica';

export type SyncStatus = 'idle' | 'syncing' | 'error';
export type SyncErrorKind =
  'unauthorized' | 'unsupported_version' | 'network' | 'server' | 'storage';

export type RejectedOperation = {
  operation_id: string;
  reason: RejectionReason['reason'];
};

export type SyncError = Error & { readonly kind: SyncErrorKind };

export function syncError(kind: SyncErrorKind, message: string, cause?: unknown): SyncError {
  // the next lines tag a plain Error with name/kind, which TS cannot express on Error alone
  const error = new Error(message, cause === undefined ? undefined : { cause }) as SyncError & {
    kind: SyncErrorKind;
  };
  error.name = 'SyncError';
  error.kind = kind;
  return error;
}

export function isSyncError(error: unknown): error is SyncError {
  return (
    error instanceof Error &&
    error.name === 'SyncError' &&
    // any Error subclass may reach here, so read the tag through a shape cast
    typeof (error as { kind?: unknown }).kind === 'string'
  );
}

export type SyncSnapshot = {
  status: SyncStatus;
  error: SyncError | null;
  rejected: readonly RejectedOperation[];
  lastSyncedAt: string | null;
};

export type SyncClientOptions = {
  replica: Replica;
  fetcher?: typeof fetch;
  baseUrl?: string;
};

export type SyncClient = {
  getSnapshot(): SyncSnapshot;
  subscribe(listener: () => void): () => void;
  sync(): Promise<void>;
  clearRejectedOperation(operationId: string): void;
  clearAllRejectedOperations(): void;
};
