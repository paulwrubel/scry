type UnauthorizedListener = () => void;

const listeners = new Set<UnauthorizedListener>();

/** Channel used to converge auth state across open tabs. */
export const AUTH_CHANNEL_NAME = 'scry-auth';

/** Auth state changes that are safe to share across tabs. */
export type AuthChannelMessage = { type: 'signed-in' } | { type: 'signed-out' };

/** Notify the Auth provider that a normal API request received a 401. */
export function notifyUnauthorized(): void {
  for (const listener of [...listeners]) {
    listener();
  }
}

export function subscribeUnauthorized(listener: UnauthorizedListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function isAuthChannelMessage(value: unknown): value is AuthChannelMessage {
  if (typeof value !== 'object' || value === null || !('type' in value)) {
    return false;
  }
  const type = (value as { type: unknown }).type;
  return type === 'signed-in' || type === 'signed-out';
}
