export function fieldErrorMessage(errors: readonly unknown[]): string | null {
  const messages = errors
    // form errors are unknown-typed; Zod issues expose a message field
    .map((error) => (typeof error === 'string' ? error : (error as { message?: string })?.message))
    .filter((message): message is string => typeof message === 'string' && message.length > 0);

  return messages.length > 0 ? messages.join(', ') : null;
}
