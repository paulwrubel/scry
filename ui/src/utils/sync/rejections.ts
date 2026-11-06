import type { RejectedOperation } from './types';

export function describeRejectionReason(reason: RejectedOperation['reason']): string {
  switch (reason) {
    case 'tombstoned':
      return 'The item was deleted elsewhere.';
    case 'not_found':
      return 'The item no longer exists.';
    case 'conflict':
      return 'A newer change took precedence.';
    case 'unsupported_version':
      return 'This app version is too old for the server.';
  }
}
