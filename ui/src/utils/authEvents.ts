/** Channel used to converge auth state across open tabs. */
export const AUTH_CHANNEL_NAME = 'scry-auth';

/** Auth state changes that are safe to share across tabs. */
export type AuthChannelMessage =
  { type: 'signed-in'; accessToken: string } | { type: 'signed-out' };

export function isAuthChannelMessage(value: unknown): value is AuthChannelMessage {
  if (typeof value !== 'object' || value === null || !('type' in value)) {
    return false;
  }
  // the guard above proves `value` is a non-null object with a `type` key.
  // casting it here only exposes the fields we probe next
  const message = value as { type: unknown; accessToken?: unknown };
  if (message.type === 'signed-out') {
    return true;
  }
  return (
    message.type === 'signed-in' &&
    typeof message.accessToken === 'string' &&
    message.accessToken !== ''
  );
}
